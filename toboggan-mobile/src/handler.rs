//! UniFFI-compatible notification handler trait and adapter.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use toboggan_client::{ErrorKind as CoreErrorKind, NotificationHandler as CoreNotificationHandler};
use toboggan_core::{
    ClientId, ClientRole as CoreClientRole, Slide as CoreSlide, State as CoreState, TalkResponse,
};
use tokio::sync::watch;
use tracing::warn;

use crate::deck::deck_snapshot;
use crate::types::{ClientRole, ConnectionStatus, ErrorKind, PresentationState, Slide};

/// Notification handler trait for Swift/Kotlin implementations.
///
/// This trait is implemented in Swift/Kotlin to receive callbacks
/// when server events occur.
#[uniffi::export(with_foreign)]
pub trait ClientNotificationHandler: Send + Sync {
    fn on_state_change(&self, state: PresentationState);
    fn on_talk_change(&self, state: PresentationState);
    fn on_connection_status_change(&self, status: ConnectionStatus);
    /// Called once the server has settled this client's role.
    ///
    /// `role` is what the app needs before it offers a control: an audience
    /// client must say it is watching rather than let the user find out by
    /// pressing something and being refused.
    fn on_registered(&self, client_id: String, role: ClientRole);
    fn on_client_connected(&self, client_id: String, name: String);
    fn on_client_disconnected(&self, client_id: String, name: String);
    /// Called when something goes wrong.
    ///
    /// `kind` is what separates a command the server refused from a server the
    /// phone cannot reach — the first belongs inline next to the controls, the
    /// second is worth interrupting for. The app used to tell them apart by
    /// searching the message for an English word.
    fn on_error(&self, kind: ErrorKind, error: String);
}

/// Adapter that converts core notifications to `UniFFI` types.
///
/// This adapter wraps a `ClientNotificationHandler` and implements
/// the core `NotificationHandler` trait, converting types as needed.
pub struct NotificationAdapter {
    inner: Arc<dyn ClientNotificationHandler>,
    slides_rx: watch::Receiver<Arc<[CoreSlide]>>,
    talk_rx: watch::Receiver<Option<TalkResponse>>,
    /// Shared with [`crate::TobogganClient`], which has no other way to answer
    /// `is_connected`: the status only ever arrives here, as a notification.
    connected: Arc<AtomicBool>,
}

impl NotificationAdapter {
    /// Create a new notification adapter.
    pub fn new(
        handler: Arc<dyn ClientNotificationHandler>,
        slides_rx: watch::Receiver<Arc<[CoreSlide]>>,
        talk_rx: watch::Receiver<Option<TalkResponse>>,
        connected: Arc<AtomicBool>,
    ) -> Self {
        Self {
            inner: handler,
            slides_rx,
            talk_rx,
            connected,
        }
    }

    /// Get slides for state conversion, each already carrying its step count.
    fn get_slides(&self) -> Vec<Slide> {
        deck_snapshot(&self.slides_rx, &self.talk_rx)
    }
}

impl CoreNotificationHandler for NotificationAdapter {
    fn on_connection_status_change(&self, status: toboggan_client::ConnectionStatus) {
        self.connected.store(
            matches!(status, toboggan_client::ConnectionStatus::Connected),
            Ordering::Relaxed,
        );
        self.inner.on_connection_status_change(status.into());
    }

    fn on_state_change(&self, state: CoreState) {
        let slides = self.get_slides();
        if slides.is_empty() {
            // Ordinary before the REST fetch lands, and indistinguishable from a
            // fetch that failed — so say it happened rather than let the app sit
            // on "Ready to start" with a green connection dot.
            warn!(?state, "Dropped a state change: no slides have arrived yet");
            return;
        }
        self.inner
            .on_state_change(PresentationState::new(&slides, &state));
    }

    fn on_talk_change(&self, state: CoreState) {
        let slides = self.get_slides();
        if slides.is_empty() {
            warn!(?state, "Dropped a talk change: no slides have arrived yet");
            return;
        }
        self.inner
            .on_talk_change(PresentationState::new(&slides, &state));
    }

    fn on_error(&self, kind: CoreErrorKind, error: String) {
        self.inner.on_error(kind.into(), error);
    }

    fn on_registered(&self, client_id: ClientId, role: CoreClientRole) {
        self.inner
            .on_registered(format!("{client_id:?}"), role.into());
    }

    fn on_client_connected(&self, client_id: ClientId, name: String) {
        self.inner
            .on_client_connected(format!("{client_id:?}"), name);
    }

    fn on_client_disconnected(&self, client_id: ClientId, name: String) {
        self.inner
            .on_client_disconnected(format!("{client_id:?}"), name);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_client::ConnectionStatus as CoreConnectionStatus;

    use super::*;

    /// A handler that does nothing, so a test can watch what the adapter does
    /// around it.
    struct Silent;

    impl ClientNotificationHandler for Silent {
        fn on_state_change(&self, _state: PresentationState) {}
        fn on_talk_change(&self, _state: PresentationState) {}
        fn on_connection_status_change(&self, _status: ConnectionStatus) {}
        fn on_registered(&self, _client_id: String, _role: ClientRole) {}
        fn on_client_connected(&self, _client_id: String, _name: String) {}
        fn on_client_disconnected(&self, _client_id: String, _name: String) {}
        fn on_error(&self, _kind: ErrorKind, _error: String) {}
    }

    fn adapter(connected: &Arc<AtomicBool>) -> NotificationAdapter {
        // The senders are dropped here on purpose: a `watch::Receiver` still
        // reads the last value it was given, and this test never sends another.
        let (_slides_tx, slides_rx) = watch::channel::<Arc<[CoreSlide]>>(Arc::from([]));
        let (_talk_tx, talk_rx) = watch::channel::<Option<TalkResponse>>(None);
        NotificationAdapter::new(Arc::new(Silent), slides_rx, talk_rx, Arc::clone(connected))
    }

    /// The flag `is_connected` reads is written here and nowhere else.
    ///
    /// The iOS test only ever asserted its *initial* value, which stayed true
    /// even with the write deleted — so nothing covered the thing that was
    /// actually broken: a status that never moved the flag.
    #[test]
    fn the_connected_flag_follows_the_status() {
        let connected = Arc::new(AtomicBool::new(false));
        let adapter = adapter(&connected);

        adapter.on_connection_status_change(CoreConnectionStatus::Connected);
        assert!(connected.load(Ordering::Relaxed), "connected");

        adapter.on_connection_status_change(CoreConnectionStatus::Closed);
        assert!(!connected.load(Ordering::Relaxed), "closed");

        adapter.on_connection_status_change(CoreConnectionStatus::Connected);
        adapter.on_connection_status_change(CoreConnectionStatus::Error {
            message: "no route to host".to_owned(),
        });
        assert!(
            !connected.load(Ordering::Relaxed),
            "an error is not connected"
        );

        adapter.on_connection_status_change(CoreConnectionStatus::Connected);
        adapter.on_connection_status_change(CoreConnectionStatus::Reconnecting {
            attempt: 1,
            max_attempt: 5,
            delay: std::time::Duration::from_secs(1),
        });
        assert!(
            !connected.load(Ordering::Relaxed),
            "reconnecting is not connected"
        );
    }
}
