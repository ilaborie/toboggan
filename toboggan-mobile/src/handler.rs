//! UniFFI-compatible notification handler trait and adapter.

use std::sync::Arc;

use toboggan_client::NotificationHandler as CoreNotificationHandler;
use toboggan_core::{ClientId, Slide as CoreSlide, State as CoreState};
use tokio::sync::watch;

use crate::types::{ConnectionStatus, Slide, State};

/// Notification handler trait for Swift/Kotlin implementations.
///
/// This trait is implemented in Swift/Kotlin to receive callbacks
/// when server events occur.
#[uniffi::export(with_foreign)]
pub trait ClientNotificationHandler: Send + Sync {
    fn on_state_change(&self, state: State);
    fn on_talk_change(&self, state: State);
    fn on_connection_status_change(&self, status: ConnectionStatus);
    fn on_registered(&self, client_id: String);
    fn on_client_connected(&self, client_id: String, name: String);
    fn on_client_disconnected(&self, client_id: String, name: String);
    fn on_error(&self, error: String);
}

/// Adapter that converts core notifications to `UniFFI` types.
///
/// This adapter wraps a `ClientNotificationHandler` and implements
/// the core `NotificationHandler` trait, converting types as needed.
pub struct NotificationAdapter {
    inner: Arc<dyn ClientNotificationHandler>,
    slides_rx: watch::Receiver<Arc<[CoreSlide]>>,
}

impl NotificationAdapter {
    /// Create a new notification adapter.
    pub fn new(
        handler: Arc<dyn ClientNotificationHandler>,
        slides_rx: watch::Receiver<Arc<[CoreSlide]>>,
    ) -> Self {
        Self {
            inner: handler,
            slides_rx,
        }
    }

    /// Get slides for state conversion.
    ///
    /// This uses `watch::Receiver::borrow()` which never fails,
    /// eliminating the previous `try_lock()` code smell.
    fn get_slides(&self) -> Vec<Slide> {
        self.slides_rx
            .borrow()
            .iter()
            .map(|slide| Slide::from(slide.clone()))
            .collect()
    }
}

impl CoreNotificationHandler for NotificationAdapter {
    fn on_connection_status_change(&self, status: toboggan_client::ConnectionStatus) {
        self.inner.on_connection_status_change(status.into());
    }

    fn on_state_change(&self, state: CoreState) {
        let slides = self.get_slides();
        if !slides.is_empty() {
            let state_uniffi = State::new(&slides, &state);
            self.inner.on_state_change(state_uniffi);
        }
    }

    fn on_talk_change(&self, state: CoreState) {
        let slides = self.get_slides();
        if !slides.is_empty() {
            let state_uniffi = State::new(&slides, &state);
            self.inner.on_talk_change(state_uniffi);
        }
    }

    fn on_error(&self, error: String) {
        self.inner.on_error(error);
    }

    fn on_registered(&self, client_id: ClientId) {
        self.inner.on_registered(format!("{client_id:?}"));
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
