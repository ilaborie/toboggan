//! UniFFI-compatible notification handler trait and adapter.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use toboggan_client::NotificationHandler as CoreNotificationHandler;
use toboggan_core::{
    ClientId, ClientRole as CoreClientRole, Slide as CoreSlide, State as CoreState, TalkResponse,
};
use tokio::sync::watch;

use crate::types::{ClientRole, ConnectionStatus, PresentationState, Slide};

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
    fn on_error(&self, error: String);
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

    /// Get slides for state conversion, using step counts from talk.
    fn get_slides(&self) -> Vec<Slide> {
        let step_counts = self
            .talk_rx
            .borrow()
            .as_ref()
            .map(|talk| talk.step_counts.clone())
            .unwrap_or_default();
        self.slides_rx
            .borrow()
            .iter()
            .enumerate()
            .map(|(i, slide)| {
                let step_count = step_counts.get(i).copied().unwrap_or(0);
                Slide::from_core_slide(slide, step_count)
            })
            .collect()
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
        if !slides.is_empty() {
            let state_uniffi = PresentationState::new(&slides, &state);
            self.inner.on_state_change(state_uniffi);
        }
    }

    fn on_talk_change(&self, state: CoreState) {
        let slides = self.get_slides();
        if !slides.is_empty() {
            let state_uniffi = PresentationState::new(&slides, &state);
            self.inner.on_talk_change(state_uniffi);
        }
    }

    fn on_error(&self, error: String) {
        self.inner.on_error(error);
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
