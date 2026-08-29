//! UniFFI-compatible Toboggan client wrapper.

#![allow(clippy::missing_panics_doc)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use toboggan_client::{TobogganClientCore, TobogganWebsocketConfig};
use toboggan_core::{RetryConfig, Secret, Slide as CoreSlide, TalkResponse};
use tokio::runtime::Runtime;
use tokio::sync::{Mutex, watch};

use crate::deck::deck_snapshot;
use crate::handler::{ClientNotificationHandler, NotificationAdapter};
use crate::types::{Command, PresentationState, Slide, Talk};

/// Client configuration for connecting to a Toboggan server.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ClientConfig {
    /// The server URL, like `http://localhost:8080`.
    ///
    /// May carry a presenter token — `http://192.168.1.10:8080/?token=s3cr3t`,
    /// exactly as the server prints it. A phone is never on the machine running
    /// the server, so without a token it registers as audience and its buttons
    /// do nothing; carrying the token in the URL means the one string the user
    /// already has to type is the whole configuration.
    pub url: String,

    /// The maximum number of retries if the connection is not working.
    ///
    /// Also the multiplier for the delay ceiling below, so raising it lengthens
    /// each wait as well as adding attempts.
    pub max_retries: u32,

    /// The delay before the first retry, doubling with jitter after that.
    ///
    /// Seconds on both host platforms: `UniFFI` maps `Duration` to Swift's
    /// `TimeInterval` and Kotlin's `java.time.Duration`, so `1.0` is one second
    /// and `1000` is a quarter of an hour.
    pub retry_delay: Duration,
}

/// Splits a configured URL into the server address and a presenter token.
///
/// Only `token` is understood; anything else in the query string is dropped
/// along with it, because what remains has to be usable as the base of both the
/// REST and WebSocket URLs.
fn split_presenter_token(url: &str) -> (String, Option<Secret>) {
    let Some((base, query)) = url.split_once('?') else {
        return (url.to_owned(), None);
    };
    // `Secret::from_query_value` decodes it, which this used not to do at all:
    // a token with a space or a `+` reached the server as different text than
    // the web client sent, and only one of them could match.
    let token = query
        .split('&')
        .find_map(|pair| Secret::from_query_value(pair.strip_prefix("token=")?));
    // A URL written as `http://host:8080/?token=…` leaves a trailing slash the
    // API paths would double up.
    (base.trim_end_matches('/').to_owned(), token)
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

    // Set by the notification adapter, which is where connection status arrives.
    connected: Arc<AtomicBool>,
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
        let (url, presenter_token) = split_presenter_token(&url);

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

        // Saturating: `Duration * u32` panics on overflow, and a panic here
        // unwinds into the host app rather than reporting a silly retry delay.
        let max_retry_delay = retry_delay.saturating_mul(max_retries);

        let websocket_config = TobogganWebsocketConfig {
            websocket_url,
            max_retries: max_retries as usize,
            retry_delay,
            max_retry_delay,
            // Built from the same numbers, so a phone that loses signal backs
            // off and jitters like every other client rather than retrying on a
            // flat timer.
            retry: RetryConfig::new(
                max_retries as usize,
                retry_delay.into(),
                max_retry_delay.into(),
                2.0,
                true,
            ),
            presenter_token,
        };

        // Create watch channels for slides and talk (shared between core and adapter)
        let (slides_tx, slides_rx) = watch::channel::<Arc<[CoreSlide]>>(Arc::from([]));
        let (talk_tx, talk_rx) = watch::channel::<Option<TalkResponse>>(None);

        // Create notification adapter with slides and talk receivers
        let connected = Arc::new(AtomicBool::new(false));
        let adapter = NotificationAdapter::new(
            handler,
            slides_rx.clone(),
            talk_rx.clone(),
            Arc::clone(&connected),
        );

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
            connected,
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

    /// Whether the WebSocket is currently connected.
    ///
    /// This used to return `true` unconditionally, which made it worse than
    /// useless: the app could not tell a live socket from one that never opened,
    /// and the iOS test that asserted a fresh client is *not* connected could
    /// never pass. The flag is written by the notification adapter, which is
    /// where connection status actually arrives.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
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
    pub fn get_state(&self) -> Option<PresentationState> {
        self.runtime.block_on(async {
            let core = self.core.lock().await;
            let state = core.get_state()?;
            let slides = deck_snapshot(&self.slides_rx, &self.talk_rx);
            if slides.is_empty() {
                return None;
            }
            Some(PresentationState::new(&slides, &state))
        })
    }

    /// Every slide, in order, in one consistent read.
    ///
    /// The app wants the deck once per connection, not a slide at a time: each
    /// call across the FFI blocks, and a caller assembling the deck itself has
    /// to guess how long it is. Asking `get_slide` for `0..talk.titles.len()`
    /// and discarding the `None`s — which is what the iOS app did — silently
    /// *shortened* the deck whenever the two channels had not both landed,
    /// shifting every slide after the gap and pointing `GoTo` at the wrong one.
    #[must_use]
    pub fn get_deck(&self) -> Vec<Slide> {
        deck_snapshot(&self.slides_rx, &self.talk_rx)
    }

    /// Get a slide by index.
    #[must_use]
    pub fn get_slide(&self, index: u32) -> Option<Slide> {
        deck_snapshot(&self.slides_rx, &self.talk_rx)
            .into_iter()
            .nth(index as usize)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape the server's homepage QR actually emits: a bare origin, a
    /// trailing slash, and the token in the query.
    #[test]
    fn a_scanned_link_splits_into_an_origin_and_a_token() {
        let (url, token) = split_presenter_token("http://192.168.1.10:8080/?token=s3cr3t");
        assert_eq!(url, "http://192.168.1.10:8080");
        assert_eq!(token, Secret::new("s3cr3t"));
    }

    /// The trailing slash has to go: what is left is the base of every REST and
    /// WebSocket path, and `…8080//api/ws` is not the same route.
    #[test]
    fn the_trailing_slash_does_not_survive() {
        let (url, _) = split_presenter_token("http://host:8080/?token=x");
        assert_eq!(url, "http://host:8080");
    }

    /// A URL with no query is the whole address, and the client is audience.
    #[test]
    fn an_address_without_a_token_keeps_all_of_itself() {
        let (url, token) = split_presenter_token("http://host:8080");
        assert_eq!(url, "http://host:8080");
        assert_eq!(token, None);
    }

    /// `token` is not required to come first.
    #[test]
    fn the_token_is_found_wherever_it_sits_in_the_query() {
        let (_, token) = split_presenter_token("http://host:8080?theme=dark&token=s3cr3t");
        assert_eq!(token, Secret::new("s3cr3t"));
    }

    /// The decode this used not to do at all. A token with a space or a `+`
    /// reached the server as different text than the web client sent, and only
    /// one of the two could match — so the phone silently registered as
    /// audience.
    #[test]
    fn an_awkward_token_is_decoded_the_way_the_server_decodes_it() {
        let (_, percent) = split_presenter_token("http://host:8080/?token=a%20b%2Bc");
        assert_eq!(percent, Secret::new("a b+c"));

        // `+` is a space in a query string, which is why the app must not send a
        // literal one for a token that contains a plus.
        let (_, plus) = split_presenter_token("http://host:8080/?token=a+b");
        assert_eq!(plus, Secret::new("a b"));
    }
}
