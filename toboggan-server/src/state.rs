use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use jiff::Timestamp;
use serde_json::json;
use toboggan_core::{ClientId, Command, Notification, Slide, SlideId, State, Talk};
use tokio::sync::{RwLock, watch};
#[derive(Clone)]
pub struct TobogganState {
    started_at: Timestamp,
    talk: Arc<Talk>,
    slides: Arc<HashMap<SlideId, Slide>>,
    slide_order: Arc<[SlideId]>,
    current_state: Arc<RwLock<State>>,
    clients: Arc<RwLock<HashMap<ClientId, watch::Sender<Notification>>>>,
}

impl TobogganState {
    #[must_use]
    pub fn new(talk: Talk) -> Self {
        let started = Timestamp::now();
        let talk = Arc::new(talk);

        let slides = talk
            .slides
            .iter()
            .map(|slide| (SlideId::next(), slide.clone()))
            .collect::<HashMap<_, _>>();
        let slide_order = slides.keys().copied().collect::<Vec<_>>();
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
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub(crate) fn health(&self) -> serde_json::Value {
        let started_at = self.started_at;
        let elasped = Timestamp::now().duration_since(started_at);
        let name = self.talk.title.to_string();

        json!({
            "status": "OK",
            "started_at": started_at,
            "elapsed": elasped,
            "talk": name
        })
    }

    pub(crate) fn talk(&self) -> &Talk {
        &self.talk
    }

    pub(crate) fn slides(&self) -> &HashMap<SlideId, Slide> {
        &self.slides
    }

    pub(crate) async fn current_state(&self) -> State {
        let state = self.current_state.read().await;

        state.clone()
    }

    pub async fn register_client(&self, client_id: ClientId) -> watch::Receiver<Notification> {
        let (tx, rx) = watch::channel(Notification::state(State::Paused {
            current: self
                .slide_order
                .first()
                .copied()
                .unwrap_or_else(SlideId::next),
            total_duration: Duration::ZERO,
        }));

        let mut clients = self.clients.write().await;
        clients.insert(client_id, tx);
        rx
    }

    pub async fn unregister_client(&self, client_id: ClientId) {
        let mut clients = self.clients.write().await;
        clients.remove(&client_id);
    }

    async fn broadcast_notification(&self, notification: &Notification) {
        let clients = self.clients.read().await;
        for (client_id, sender) in clients.iter() {
            if sender.send(notification.clone()).is_err() {
                tracing::warn!(
                    ?client_id,
                    "Failed to send notification to client, client may have disconnected"
                );
            }
        }
    }

    pub async fn handle_command(&self, command: &Command) -> Notification {
        let mut state = self.current_state.write().await;
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

        // Broadcast the notification to all registered clients
        self.broadcast_notification(&notification).await;

        notification
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
