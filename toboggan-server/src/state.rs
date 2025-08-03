use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{RwLock, watch};
use tracing::{info, warn};

use toboggan_core::{
    ClientId, Command, Duration, Notification, Slide, SlideId, State, Talk, Timestamp,
};

use crate::{ApiError, HealthResponse, HealthResponseStatus};

#[derive(Clone)]
pub struct TobogganState {
    started_at: Timestamp,
    talk: Talk,
    slides: HashMap<SlideId, Slide>,
    slide_order: Vec<SlideId>,
    current_state: Arc<RwLock<State>>,
    clients: Arc<DashMap<ClientId, watch::Sender<Notification>>>,
    max_clients: usize,
}

impl TobogganState {
    /// Creates a new `TobogganState` from a talk.
    ///
    /// # Panics
    ///
    /// Panics if the talk has no slides.
    #[must_use]
    pub fn new(talk: Talk, max_clients: usize) -> Self {
        let started = Timestamp::now();

        let slide_data: Vec<_> = talk
            .slides
            .iter()
            .map(|slide| (SlideId::next(), slide.clone()))
            .collect();
        assert!(
            !slide_data.is_empty(),
            "Empty talk, need at least one slide, got {talk:#?}"
        );

        info!(
            "\n=== Slides ===\n{}",
            slide_data
                .iter()
                .map(|(id, slide)| format!("[{id}]: {slide}"))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let slide_order = slide_data.iter().map(|(id, _)| *id).collect();
        let slides = slide_data.into_iter().collect();
        let current_state = State::default(); // Init state
        let current_state = Arc::new(RwLock::new(current_state));

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

    pub(crate) fn health(&self) -> HealthResponse {
        let status = HealthResponseStatus::Ok;
        let started_at = self.started_at;
        let elapsed = started_at.elapsed();
        let name = self.talk.title.to_string();
        let active_clients = self.clients.len();

        HealthResponse {
            status,
            started_at,
            elapsed,
            talk: name,
            active_clients,
        }
    }

    pub(crate) fn talk(&self) -> &Talk {
        &self.talk
    }

    pub(crate) fn slides(&self) -> Vec<Slide> {
        self.talk().slides.clone()
    }

    pub(crate) fn slide_by_id(&self, id: SlideId) -> Option<Slide> {
        self.slides.get(&id).cloned()
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
    ) -> Result<watch::Receiver<Notification>, ApiError> {
        // Clean up disconnected clients first
        self.cleanup_disconnected_clients();

        // Check client limit
        if self.clients.len() >= self.max_clients {
            return Err(ApiError::TooManyClients);
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

    pub async fn cleanup_clients_task(&self, cleanup_interval: std::time::Duration) {
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

    #[allow(clippy::expect_used)]
    fn command_first(&self, state: &mut State) -> Notification {
        match state {
            State::Init => {
                // In Init state, go to first slide and start running
                *state = State::Running {
                    since: Timestamp::now(),
                    current: self.slide_order.first().copied().expect("a first slide"),
                    total_duration: Duration::default(),
                };
            }
            State::Paused { .. } => {
                // In Paused state, go to first slide and start running with reset timestamp
                if let Some(&first_slide) = self.slide_order.first() {
                    *state = State::Running {
                        since: Timestamp::now(),
                        current: first_slide,
                        total_duration: Duration::default(),
                    };
                }
            }
            _ => {
                // Normal behavior for Running/Done states - reset timestamp when going to first
                if !state.is_first_slide(&self.slide_order) {
                    if let Some(&first_slide) = self.slide_order.first() {
                        *state = State::Running {
                            since: Timestamp::now(),
                            current: first_slide,
                            total_duration: Duration::default(),
                        };
                    }
                }
            }
        }
        Notification::state(state.clone())
    }

    fn command_last(&self, state: &mut State) -> Notification {
        match state {
            State::Init => {
                // In Init state, go to first slide and start running (last command from init goes to first)
                if let Some(&first_slide) = self.slide_order.first() {
                    *state = State::Running {
                        since: Timestamp::now(),
                        current: first_slide,
                        total_duration: Duration::ZERO,
                    };
                }
            }
            State::Paused { .. } => {
                // In Paused state, navigate and start running (unless already on last slide)
                if let Some(&last_slide) = self.slide_order.last() {
                    if state.is_last_slide(&self.slide_order) {
                        // Stay paused if already on last slide (no movement)
                        state.update_slide(last_slide);
                    } else {
                        // Go to last slide and start running
                        *state = State::Running {
                            since: Timestamp::now(),
                            current: last_slide,
                            total_duration: state.calculate_total_duration(),
                        };
                    }
                }
            }
            _ => {
                // Normal behavior for Running/Done states
                if !state.is_last_slide(&self.slide_order) {
                    if let Some(&last_slide) = self.slide_order.last() {
                        state.update_slide(last_slide);
                    }
                }
            }
        }
        Notification::state(state.clone())
    }

    fn command_goto(&self, state: &mut State, slide_id: SlideId) -> Notification {
        if self.slide_order.contains(&slide_id) {
            match state {
                State::Init => {
                    // In Init state, go to specified slide and start running
                    *state = State::Running {
                        since: Timestamp::now(),
                        current: slide_id,
                        total_duration: Duration::ZERO,
                    };
                }
                State::Paused { .. } => {
                    // In Paused state, navigate and start running (unless going to last slide)
                    if state.is_last_slide(&self.slide_order)
                        && Some(&slide_id) == self.slide_order.last()
                    {
                        // Stay paused if going to last slide and already on last slide
                        state.update_slide(slide_id);
                    } else {
                        // Go to specified slide and start running
                        *state = State::Running {
                            since: Timestamp::now(),
                            current: slide_id,
                            total_duration: state.calculate_total_duration(),
                        };
                    }
                }
                _ => {
                    // Normal behavior for Running/Done states
                    state.update_slide(slide_id);
                }
            }
            Notification::state(state.clone())
        } else {
            Notification::error(format!("Slide with id {slide_id:?} not found"))
        }
    }

    fn command_next(&self, state: &mut State) -> Notification {
        match state {
            State::Init => {
                // In Init state, go to first slide and start running
                if let Some(&first_slide) = self.slide_order.first() {
                    *state = State::Running {
                        since: Timestamp::now(),
                        current: first_slide,
                        total_duration: Duration::ZERO,
                    };
                } else {
                    warn!("No slides in slide_order when handling Next from Init");
                }
            }
            State::Paused { .. } => {
                // In Paused state, navigate and start running
                if let Some(next_slide) = state.next(&self.slide_order) {
                    // Always start running when we can navigate
                    *state = State::Running {
                        since: Timestamp::now(),
                        current: next_slide,
                        total_duration: state.calculate_total_duration(),
                    };
                }
                // If no next slide available (already on last slide), stay paused
                // No state change needed
            }
            _ => {
                // Normal behavior for Running/Done states
                if let Some(current) = state.current() {
                    if let Some(next_slide) = state.next(&self.slide_order) {
                        state.update_slide(next_slide);
                    } else if state.is_last_slide(&self.slide_order) {
                        // We're at the last slide, mark as done
                        let total_duration = state.calculate_total_duration();
                        *state = State::Done {
                            current,
                            total_duration,
                        };
                    }
                } else if let Some(current) = self.slide_order.first().copied() {
                    *state = State::Running {
                        since: Timestamp::now(),
                        current,
                        total_duration: Duration::ZERO,
                    }
                } else {
                    warn!("No first slide to start the talk");
                }
            }
        }
        Notification::state(state.clone())
    }

    fn command_previous(&self, state: &mut State) -> Notification {
        match state {
            State::Init => {
                // In Init state, go to first slide and start running
                if let Some(&first_slide) = self.slide_order.first() {
                    *state = State::Running {
                        since: Timestamp::now(),
                        current: first_slide,
                        total_duration: Duration::ZERO,
                    };
                }
            }
            State::Paused { .. } => {
                // In Paused state, navigate and start running
                if let Some(prev_slide) = state.previous(&self.slide_order) {
                    *state = State::Running {
                        since: Timestamp::now(),
                        current: prev_slide,
                        total_duration: state.calculate_total_duration(),
                    };
                }
            }
            _ => {
                // Normal behavior for Running/Done states
                if let Some(prev_slide) = state.previous(&self.slide_order) {
                    state.update_slide(prev_slide);
                }
            }
        }
        Notification::state(state.clone())
    }

    fn command_pause(state: &mut State) -> Notification {
        // Skip if already paused or done
        if let State::Running { current, .. } = *state {
            let total_duration = state.calculate_total_duration();
            *state = State::Paused {
                current: Some(current),
                total_duration,
            };
        }
        Notification::state(state.clone())
    }

    fn command_resume(state: &mut State) -> Notification {
        // Skip if already running or done
        if matches!(*state, State::Paused { .. }) {
            state.auto_resume();
        }
        Notification::state(state.clone())
    }

    fn command_blink() -> Notification {
        // Blink command creates a blink notification without changing state
        Notification::blink()
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
            Command::First => self.command_first(&mut state),
            Command::Last => self.command_last(&mut state),
            Command::GoTo(slide_id) => self.command_goto(&mut state, *slide_id),
            Command::Next => self.command_next(&mut state),
            Command::Previous => self.command_previous(&mut state),
            Command::Pause => Self::command_pause(&mut state),
            Command::Resume => Self::command_resume(&mut state),
            Command::Blink => Self::command_blink(),
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
