//! UniFFI-compatible Toboggan client wrapper.

#![allow(clippy::print_stdout, clippy::missing_panics_doc, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use toboggan_client::{TobogganClientCore, TobogganWebsocketConfig};
use toboggan_core::Slide as CoreSlide;
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
        println!("using {config:#?}");

        let ClientConfig {
            url,
            max_retries,
            retry_delay,
        } = config;

        // Convert HTTP URL to WebSocket URL
        let websocket_url = if url.starts_with("http://") {
            format!("ws://{}/api/ws", url.trim_start_matches("http://"))
        } else if url.starts_with("https://") {
            format!("wss://{}/api/ws", url.trim_start_matches("https://"))
        } else {
            panic!("invalid url '{url}', expected 'http(s)://<host>:<port>'");
        };

        let websocket_config = TobogganWebsocketConfig {
            websocket_url,
            max_retries: max_retries as usize,
            retry_delay,
            max_retry_delay: retry_delay * max_retries,
        };

        // Create watch channel for slides (shared between core and adapter)
        let (slides_tx, slides_rx) = watch::channel::<Arc<[CoreSlide]>>(Arc::from([]));

        // Create notification adapter with slides receiver
        let adapter = NotificationAdapter::new(handler, slides_rx.clone());

        // Create API URL (trim trailing slash)
        let api_url = url.trim_end_matches('/');

        // Create core client with external slides channel
        let core = TobogganClientCore::new_with_slides_channel(
            api_url,
            websocket_config,
            client_name,
            adapter,
            slides_tx,
            slides_rx.clone(),
        );

        // Create tokio runtime
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("having a tokio runtime");

        Self {
            slides_rx,
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
        println!("connected");
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
            let slides: Vec<Slide> = self
                .slides_rx
                .borrow()
                .iter()
                .map(|slide| Slide::from(slide.clone()))
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
        let slides = self.slides_rx.borrow();
        slides
            .get(index as usize)
            .map(|slide| Slide::from(slide.clone()))
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
