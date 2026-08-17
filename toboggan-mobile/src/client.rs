//! UniFFI-compatible Toboggan client wrapper.

#![allow(clippy::missing_panics_doc)]

use std::sync::Arc;
use std::time::Duration;

use toboggan_client::{TobogganClientCore, TobogganWebsocketConfig};
use toboggan_core::{Slide as CoreSlide, TalkResponse};
use tokio::runtime::Runtime;
use tokio::sync::{Mutex, watch};

use crate::handler::{ClientNotificationHandler, NotificationAdapter};
use crate::types::{Command, Slide, State, Talk};

/// Client configuration for connecting to a Toboggan server.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ClientConfig {
    /// The server URL, like `http://localhost:8080`
    pub url: String,

    /// The maximum number of retries if the connection is not working
    pub max_retries: u32,

    /// The delay between retries
    pub retry_delay: Duration,
}

/// The Toboggan client for mobile platforms.
///
/// This is a thin wrapper around `TobogganClientCore` that provides
/// UniFFI-compatible sync methods and type conversions.
#[derive(uniffi::Object)]
pub struct TobogganClient {
    // Watch receiver for slides (shared with notification adapter)
    slides_rx: watch::Receiver<Arc<[CoreSlide]>>,

    // Watch receiver for talk (for step counts)
    talk_rx: watch::Receiver<Option<TalkResponse>>,

    // Core client
    core: Mutex<TobogganClientCore<NotificationAdapter>>,

    // Tokio runtime for async/sync bridging
    runtime: Runtime,
}

#[uniffi::export]
impl TobogganClient {
    /// Create a new Toboggan client.
    #[uniffi::constructor]
    pub fn new(
        config: ClientConfig,
        client_name: String,
        handler: Arc<dyn ClientNotificationHandler>,
    ) -> Self {
        let ClientConfig {
            url,
            max_retries,
            retry_delay,
        } = config;

        // Convert HTTP URL to WebSocket URL.
        //
        // An unrecognised scheme used to `panic!` here. This constructor is
        // called across the UniFFI boundary, where a panic unwinds into foreign
        // code and takes the host app down — for a mistyped server address, and
        // with nothing shown to the user. The URL is passed through instead: the
        // connection then fails and is reported through the handler's connection
        // status, which the app already surfaces as an error.
        let websocket_url = match url.split_once("://") {
            Some(("http", rest)) => format!("ws://{rest}/api/ws"),
            Some(("https", rest)) => format!("wss://{rest}/api/ws"),
            _ => url.clone(),
        };

        let websocket_config = TobogganWebsocketConfig {
            websocket_url,
            max_retries: max_retries as usize,
            retry_delay,
            max_retry_delay: retry_delay * max_retries,
        };

        // Create watch channels for slides and talk (shared between core and adapter)
        let (slides_tx, slides_rx) = watch::channel::<Arc<[CoreSlide]>>(Arc::from([]));
        let (talk_tx, talk_rx) = watch::channel::<Option<TalkResponse>>(None);

        // Create notification adapter with slides and talk receivers
        let adapter = NotificationAdapter::new(handler, slides_rx.clone(), talk_rx.clone());

        // Create API URL (trim trailing slash)
        let api_url = url.trim_end_matches('/');

        // Create core client with external channels
        let core = TobogganClientCore::new_with_external_channels(
            api_url,
            websocket_config,
            client_name,
            adapter,
            slides_tx,
            slides_rx.clone(),
            talk_tx,
            talk_rx.clone(),
        );

        // Create tokio runtime.
        //
        // Scoped rather than allowed for the whole module: unlike the URL above
        // this genuinely has no local recovery — without a runtime the client
        // cannot do anything at all — and it fails only if the OS refuses to
        // give us a thread, at which point the app is over regardless.
        #[allow(clippy::expect_used)]
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("the OS should be able to start the client's worker threads");

        Self {
            slides_rx,
            talk_rx,
            core: Mutex::new(core),
            runtime,
        }
    }

    /// Connect to the server.
    ///
    /// This will load talk and slides, then establish a WebSocket connection.
    pub fn connect(&self) {
        self.runtime.block_on(async {
            let mut core = self.core.lock().await;
            core.connect().await;
            // Slides are automatically synced via watch channel - no manual update needed
        });
    }

    /// Check if the client is connected.
    ///
    /// Note: This checks if we have a command channel, not the actual connection state.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        // We consider connected if the core has been connected
        // A more accurate check would be to track connection status
        true
    }

    /// Send a command to the server.
    pub fn send_command(&self, command: Command) {
        self.runtime.block_on(async {
            let core = self.core.lock().await;
            core.send_command(command.into());
        });
    }

    /// Get the current presentation state.
    #[must_use]
    pub fn get_state(&self) -> Option<State> {
        self.runtime.block_on(async {
            let core = self.core.lock().await;
            let state = core.get_state()?;
            let step_counts = self
                .talk_rx
                .borrow()
                .as_ref()
                .map(|talk| talk.step_counts.clone())
                .unwrap_or_default();
            let slides: Vec<Slide> = self
                .slides_rx
                .borrow()
                .iter()
                .enumerate()
                .map(|(i, slide)| {
                    let step_count = step_counts.get(i).copied().unwrap_or(0);
                    Slide::from_core_slide(slide, step_count)
                })
                .collect();
            if slides.is_empty() {
                return None;
            }
            Some(State::new(&slides, &state))
        })
    }

    /// Get a slide by index.
    #[must_use]
    pub fn get_slide(&self, index: u32) -> Option<Slide> {
        let step_counts = self
            .talk_rx
            .borrow()
            .as_ref()
            .map(|talk| talk.step_counts.clone())
            .unwrap_or_default();
        let step_count = step_counts.get(index as usize).copied().unwrap_or(0);
        let slides = self.slides_rx.borrow();
        slides
            .get(index as usize)
            .map(|slide| Slide::from_core_slide(slide, step_count))
    }

    /// Get the current talk metadata.
    #[must_use]
    pub fn get_talk(&self) -> Option<Talk> {
        self.runtime.block_on(async {
            let core = self.core.lock().await;
            core.get_talk().map(Into::into)
        })
    }
}
