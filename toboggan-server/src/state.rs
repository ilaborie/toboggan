use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use jiff::Timestamp;
use tokio::sync::{RwLock, watch};
use tracing::info;

use toboggan_core::{ClientId, Command, Notification, Slide, SlideId, State, Talk};

#[derive(Clone)]
pub struct TobogganState {
    started_at: Timestamp,
    talk: Arc<Talk>,
    slides: Arc<HashMap<SlideId, Slide>>,
    slide_order: Arc<[SlideId]>,
    current_state: Arc<RwLock<State>>,
    clients: Arc<DashMap<ClientId, watch::Sender<Notification>>>,
    max_clients: usize,
}

impl TobogganState {
    #[must_use]
    pub fn new(talk: Talk, max_clients: usize) -> Self {
        let started = Timestamp::now();
        let talk = Arc::new(talk);

        let slide_data: Vec<_> = talk
            .slides
            .iter()
            .map(|slide| (SlideId::next(), slide.clone()))
            .collect();

        info!(
            "\n=== Slides ===\n{}",
            slide_data
                .iter()
                .map(|(id, slide)| format!("[{id}]: {slide}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let slides = slide_data.iter().cloned().collect::<HashMap<_, _>>();
        let slide_order: Vec<SlideId> = slide_data.iter().map(|(id, _)| *id).collect();
        let first_slide = slide_order.first().copied().unwrap_or_else(SlideId::next);
        let slides = Arc::new(slides);

        let current_state = State::Paused {
            current: first_slide,
            total_duration: Duration::ZERO,
        };
        let current_state = Arc::new(RwLock::new(current_state));
        let slide_order: Arc<[SlideId]> = slide_order.into();

        Self {
            started_at: started,
            talk,
            slides,
            slide_order,
            current_state,
            clients: Arc::new(DashMap::new()),
            max_clients,
        }
    }

    pub(crate) fn health(&self) -> crate::router::HealthResponse {
        use crate::router::HealthResponse;

        let started_at = self.started_at;
        let elapsed = Timestamp::now().duration_since(started_at);
        let name = self.talk.title.to_string();
        let active_clients = self.clients.len();

        HealthResponse {
            status: "OK".to_string(),
            started_at,
            elapsed,
            talk: name,
            active_clients,
        }
    }

    pub(crate) fn talk(&self) -> &Talk {
        &self.talk
    }

    pub(crate) fn slides_arc(&self) -> &Arc<HashMap<SlideId, Slide>> {
        &self.slides
    }

    pub(crate) async fn current_state(&self) -> State {
        let state = self.current_state.read().await;

        state.clone()
    }

    /// Registers a new client for notifications
    ///
    /// # Errors
    /// Returns an error if the maximum number of clients is exceeded
    pub async fn register_client(
        &self,
        client_id: ClientId,
    ) -> Result<watch::Receiver<Notification>, &'static str> {
        // Clean up disconnected clients first
        self.cleanup_disconnected_clients();

        // Check client limit
        if self.clients.len() >= self.max_clients {
            return Err("Maximum number of clients exceeded");
        }

        // Get the current state to send to the new client
        let current_state = self.current_state.read().await;
        let initial_notification = Notification::state(current_state.clone());

        let (tx, rx) = watch::channel(initial_notification);

        self.clients.insert(client_id, tx);
        tracing::info!(
            ?client_id,
            active_clients = self.clients.len(),
            "Client registered"
        );

        Ok(rx)
    }

    fn cleanup_disconnected_clients(&self) {
        let initial_count = self.clients.len();

        self.clients.retain(|client_id, tx| {
            let is_connected = !tx.is_closed();
            if !is_connected {
                tracing::debug!(?client_id, "Removing disconnected client");
            }
            is_connected
        });

        let removed_count = initial_count - self.clients.len();
        if removed_count > 0 {
            tracing::info!(
                removed_count,
                active_clients = self.clients.len(),
                "Cleaned up disconnected clients"
            );
        }
    }

    pub async fn cleanup_clients_task(&self, cleanup_interval: Duration) {
        let mut interval = tokio::time::interval(cleanup_interval);

        loop {
            interval.tick().await;
            self.cleanup_disconnected_clients();
        }
    }

    pub fn unregister_client(&self, client_id: ClientId) {
        self.clients.remove(&client_id);
        tracing::info!(
            ?client_id,
            active_clients = self.clients.len(),
            "Client unregistered"
        );
    }

    fn broadcast_notification(&self, notification: &Notification) {
        for client_entry in self.clients.iter() {
            let client_id = client_entry.key();
            let sender = client_entry.value();
            if sender.send(notification.clone()).is_err() {
                tracing::warn!(
                    ?client_id,
                    "Failed to send notification to client, client may have disconnected"
                );
            }
        }
    }

    pub async fn handle_command(&self, command: &Command) -> Notification {
        let start_time = std::time::Instant::now();
        let mut state = self.current_state.write().await;

        // Process command and create notification atomically
        let notification = match command {
            Command::Register { .. } => {
                // Registration is handled separately via WebSocket
                Notification::state(state.clone())
            }
            Command::Unregister { .. } => {
                // Unregistration is handled separately via WebSocket
                Notification::state(state.clone())
            }
            Command::First => {
                if !state.is_first_slide(&self.slide_order) {
                    if let Some(&first_slide) = self.slide_order.first() {
                        state.update_slide(first_slide);
                    }
                }
                Notification::state(state.clone())
            }
            Command::Last => {
                if !state.is_last_slide(&self.slide_order) {
                    if let Some(&last_slide) = self.slide_order.last() {
                        state.update_slide(last_slide);
                    }
                }
                Notification::state(state.clone())
            }
            Command::GoTo(slide_id) => {
                if self.slide_order.contains(slide_id) {
                    state.update_slide(*slide_id);
                    Notification::state(state.clone())
                } else {
                    return Notification::error(format!("Slide with id {slide_id:?} not found"));
                }
            }
            Command::Next => {
                if let Some(next_slide) = state.next(&self.slide_order) {
                    state.update_slide(next_slide);
                } else if state.is_last_slide(&self.slide_order) {
                    // We're at the last slide, mark as done
                    let total_duration = state.calculate_total_duration();
                    *state = State::Done {
                        current: state.current(),
                        total_duration,
                    };
                }
                Notification::state(state.clone())
            }
            Command::Previous => {
                if let Some(prev_slide) = state.previous(&self.slide_order) {
                    state.update_slide(prev_slide);
                }
                Notification::state(state.clone())
            }
            Command::Pause => {
                // Skip if already paused or done
                if matches!(*state, State::Running { .. }) {
                    let total_duration = state.calculate_total_duration();
                    *state = State::Paused {
                        current: state.current(),
                        total_duration,
                    };
                }
                Notification::state(state.clone())
            }
            Command::Resume => {
                // Skip if already running or done
                if matches!(*state, State::Paused { .. }) {
                    state.auto_resume();
                }
                Notification::state(state.clone())
            }
            Command::Ping => Notification::pong(),
        };

        // Broadcast the notification while still holding the write lock to ensure atomicity
        self.broadcast_notification(&notification);

        // Drop the write lock explicitly after broadcasting
        drop(state);

        tracing::debug!(
            ?command,
            duration_ms = start_time.elapsed().as_millis(),
            active_clients = self.clients.len(),
            "Command handled and broadcast completed"
        );

        notification
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
