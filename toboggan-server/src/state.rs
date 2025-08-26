use std::sync::Arc;

use dashmap::DashMap;
use toboggan_core::{ClientId, Command, Duration, Notification, Slide, State, Talk, Timestamp};
use tokio::sync::{RwLock, watch};
use tracing::{info, warn};

use crate::{ApiError, HealthResponse, HealthResponseStatus};

#[derive(Clone)]
pub struct TobogganState {
    started_at: Timestamp,
    talk: Talk,
    current_state: Arc<RwLock<State>>,
    clients: Arc<DashMap<ClientId, watch::Sender<Notification>>>,
    max_clients: usize,
}

impl TobogganState {
    /// # Panics
    /// Panics if talk has no slides
    #[must_use]
    pub fn new(talk: Talk, max_clients: usize) -> Self {
        let started = Timestamp::now();

        assert!(
            !talk.slides.is_empty(),
            "Empty talk, need at least one slide, got {talk:#?}"
        );

        info!(
            "\n=== Slides ===\n{}",
            talk.slides
                .iter()
                .enumerate()
                .map(|(index, slide)| format!("[{index}]: {slide}"))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let current_state = State::default(); // Init state
        let current_state = Arc::new(RwLock::new(current_state));

        Self {
            started_at: started,
            talk,
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

    pub(crate) fn slides(&self) -> &[Slide] {
        &self.talk.slides
    }

    pub(crate) fn slide_by_index(&self, index: usize) -> Option<Slide> {
        self.talk.slides.get(index).cloned()
    }

    fn total_slides(&self) -> usize {
        self.talk.slides.len()
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
        self.cleanup_disconnected_clients();

        if self.clients.len() >= self.max_clients {
            return Err(ApiError::TooManyClients);
        }

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

    fn transition_to_running(state: &mut State, slide_index: usize) {
        let total_duration = match state {
            State::Init => Duration::ZERO,
            _ => state.calculate_total_duration(),
        };

        *state = State::Running {
            since: Timestamp::now(),
            current: slide_index,
            total_duration,
        };
    }

    fn transition_to_running_reset_duration(state: &mut State, slide_index: usize) {
        *state = State::Running {
            since: Timestamp::now(),
            current: slide_index,
            total_duration: Duration::ZERO,
        };
    }

    fn command_first(&self, state: &mut State) -> Notification {
        let total_slides = self.total_slides();
        if total_slides == 0 {
            return Notification::error("No slides available".to_string());
        }

        match state {
            State::Init | State::Paused { .. } => {
                Self::transition_to_running_reset_duration(state, 0);
            }
            _ => {
                if !state.is_first_slide(total_slides) {
                    Self::transition_to_running_reset_duration(state, 0);
                }
            }
        }
        Notification::state(state.clone())
    }

    fn command_last(&self, state: &mut State) -> Notification {
        let total_slides = self.total_slides();
        if total_slides == 0 {
            return Notification::error("No slides available".to_string());
        }

        let last_index = total_slides - 1;

        match state {
            State::Init => {
                Self::transition_to_running(state, last_index);
            }
            State::Paused { .. } => {
                if state.is_last_slide(total_slides) {
                    state.update_slide(last_index);
                } else {
                    Self::transition_to_running(state, last_index);
                }
            }
            _ => {
                if !state.is_last_slide(total_slides) {
                    state.update_slide(last_index);
                }
            }
        }
        Notification::state(state.clone())
    }

    fn command_goto(&self, state: &mut State, slide_index: usize) -> Notification {
        let total_slides = self.total_slides();
        if slide_index >= total_slides {
            return Notification::error(format!(
                "Slide index {slide_index} not found, total slides: {total_slides}"
            ));
        }

        match state {
            State::Init => {
                Self::transition_to_running(state, slide_index);
            }
            State::Paused { .. } => {
                if state.is_last_slide(total_slides) && slide_index == total_slides - 1 {
                    state.update_slide(slide_index);
                } else {
                    Self::transition_to_running(state, slide_index);
                }
            }
            _ => {
                state.update_slide(slide_index);
            }
        }
        Notification::state(state.clone())
    }

    fn command_next(&self, state: &mut State) -> Notification {
        let total_slides = self.total_slides();
        if total_slides == 0 {
            warn!("No slides available when handling Next");
            return Notification::error("No slides available".to_string());
        }

        match state {
            State::Init => {
                Self::transition_to_running(state, 0);
            }
            State::Paused { .. } => {
                if let Some(next_slide) = state.next(total_slides) {
                    Self::transition_to_running(state, next_slide);
                }
            }
            _ => {
                if let Some(current) = state.current() {
                    if let Some(next_slide) = state.next(total_slides) {
                        state.update_slide(next_slide);
                    } else if state.is_last_slide(total_slides) {
                        let total_duration = state.calculate_total_duration();
                        *state = State::Done {
                            current,
                            total_duration,
                        };
                    }
                } else {
                    // No current slide, start from the beginning
                    Self::transition_to_running(state, 0);
                }
            }
        }
        Notification::state(state.clone())
    }

    fn command_previous(&self, state: &mut State) -> Notification {
        let total_slides = self.total_slides();
        if total_slides == 0 {
            return Notification::error("No slides available".to_string());
        }

        match state {
            State::Init => {
                Self::transition_to_running(state, 0);
            }
            State::Paused { .. } => {
                if let Some(prev_slide) = state.previous(total_slides) {
                    Self::transition_to_running(state, prev_slide);
                }
            }
            _ => {
                if let Some(prev_slide) = state.previous(total_slides) {
                    state.update_slide(prev_slide);
                }
            }
        }
        Notification::state(state.clone())
    }

    fn command_pause(state: &mut State) -> Notification {
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
        if matches!(*state, State::Paused { .. }) {
            state.auto_resume();
        }
        Notification::state(state.clone())
    }

    fn command_blink() -> Notification {
        Notification::blink()
    }

    pub async fn handle_command(&self, command: &Command) -> Notification {
        let start_time = std::time::Instant::now();
        let mut state = self.current_state.write().await;

        #[allow(clippy::match_same_arms)]
        let notification = match command {
            Command::Register { .. } => Notification::state(state.clone()),
            Command::Unregister { .. } => Notification::state(state.clone()),
            Command::First => self.command_first(&mut state),
            Command::Last => self.command_last(&mut state),
            Command::GoTo { slide } => self.command_goto(&mut state, *slide),
            Command::Next => self.command_next(&mut state),
            Command::Previous => self.command_previous(&mut state),
            Command::Pause => Self::command_pause(&mut state),
            Command::Resume => Self::command_resume(&mut state),
            Command::Blink => Self::command_blink(),
            Command::Ping => Notification::pong(),
        };

        self.broadcast_notification(&notification);
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
