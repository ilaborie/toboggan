use std::sync::Arc;

use anyhow::bail;
use dashmap::DashMap;
use toboggan_core::{ClientId, Command, Duration, Notification, Slide, State, Talk, Timestamp};
use tokio::sync::{RwLock, watch};
use tracing::{info, warn};

use crate::{ApiError, HealthResponse, HealthResponseStatus};

#[derive(Clone)]
pub struct TobogganState {
    started_at: Timestamp,
    talk: Arc<RwLock<Talk>>,
    current_state: Arc<RwLock<State>>,
    clients: Arc<DashMap<ClientId, watch::Sender<Notification>>>,
    max_clients: usize,
}

impl TobogganState {
    pub fn new(talk: Talk, max_clients: usize) -> anyhow::Result<Self> {
        let started = Timestamp::now();

        if talk.slides.is_empty() {
            bail!("Empty talk, need at least one slide, got {talk:#?}");
        }

        info!(
            "\n=== Slides ===\n{}",
            talk.slides
                .iter()
                .enumerate()
                .map(|(index, slide)| format!("[{index:02}] {slide}"))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let current_state = State::default();
        let current_state = Arc::new(RwLock::new(current_state));

        Ok(Self {
            started_at: started,
            talk: Arc::new(RwLock::new(talk)),
            current_state,
            clients: Arc::new(DashMap::new()),
            max_clients,
        })
    }

    pub(crate) async fn health(&self) -> HealthResponse {
        let status = HealthResponseStatus::Ok;
        let started_at = self.started_at;
        let elapsed = started_at.elapsed();
        let talk = self.talk.read().await;
        let name = talk.title.clone();
        let active_clients = self.clients.len();

        HealthResponse {
            status,
            started_at,
            elapsed,
            talk: name,
            active_clients,
        }
    }

    pub(crate) async fn talk(&self) -> Talk {
        self.talk.read().await.clone()
    }

    pub(crate) async fn slides(&self) -> Vec<Slide> {
        let talk = self.talk.read().await;
        talk.slides.clone()
    }

    pub(crate) async fn slide_by_index(&self, index: usize) -> Option<Slide> {
        let talk = self.talk.read().await;
        talk.slides.get(index).cloned()
    }

    async fn total_slides(&self) -> usize {
        let talk = self.talk.read().await;
        talk.slides.len()
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

        if !self.has_capacity() {
            return Err(ApiError::TooManyClients);
        }

        let initial_notification = self.create_initial_notification().await;

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

    fn transition_to_running(state: &mut State, slide_index: usize, reset_duration: bool) {
        let total_duration = if reset_duration {
            Duration::ZERO
        } else {
            match state {
                State::Init => Duration::ZERO,
                _ => state.calculate_total_duration(),
            }
        };

        *state = State::Running {
            since: Timestamp::now(),
            current: slide_index,
            current_step: 0,
            total_duration,
        };
    }

    async fn command_first(&self, state: &mut State) -> Notification {
        let total_slides = self.total_slides().await;
        if total_slides == 0 {
            return Notification::error("No slides available".to_string());
        }

        let should_transition = matches!(state, State::Init | State::Paused { .. })
            || !state.is_first_slide(total_slides);

        if should_transition {
            Self::transition_to_running(state, 0, true);
        }

        Notification::state(state.clone())
    }

    async fn command_last(&self, state: &mut State) -> Notification {
        let total_slides = self.total_slides().await;
        if total_slides == 0 {
            return Notification::error("No slides available".to_string());
        }

        let last_index = total_slides - 1;
        Self::navigate_to_slide(state, last_index, total_slides);
        Notification::state(state.clone())
    }

    async fn command_goto(&self, state: &mut State, slide_index: usize) -> Notification {
        let total_slides = self.total_slides().await;
        if slide_index >= total_slides {
            return Notification::error(format!(
                "Slide index {slide_index} not found, total slides: {total_slides}"
            ));
        }

        Self::navigate_to_slide(state, slide_index, total_slides);
        Notification::state(state.clone())
    }

    async fn command_next(&self, state: &mut State) -> Notification {
        let total_slides = self.total_slides().await;
        if total_slides == 0 {
            warn!("No slides available when handling Next");
            return Notification::error("No slides available".to_string());
        }

        match state {
            State::Init => Self::transition_to_running(state, 0, false),
            State::Paused { .. } => {
                if let Some(next_slide) = state.next(total_slides) {
                    Self::transition_to_running(state, next_slide, false);
                }
            }
            _ => Self::handle_next_in_running_state(state, total_slides),
        }

        Notification::state(state.clone())
    }

    async fn command_previous(&self, state: &mut State) -> Notification {
        let total_slides = self.total_slides().await;
        if total_slides == 0 {
            return Notification::error("No slides available".to_string());
        }

        match state {
            State::Init => Self::transition_to_running(state, 0, false),
            State::Paused { .. } => {
                if let Some(prev_slide) = state.previous(total_slides) {
                    Self::transition_to_running(state, prev_slide, false);
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
        if let State::Running {
            current,
            current_step,
            ..
        } = *state
        {
            let total_duration = state.calculate_total_duration();
            *state = State::Paused {
                current: Some(current),
                current_step,
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
        Notification::BLINK
    }

    async fn command_next_step(&self, state: &mut State) -> Notification {
        let Some(current_slide_index) = state.current() else {
            return Notification::state(state.clone());
        };

        let Some(slide) = self.slide_by_index(current_slide_index).await else {
            return Notification::state(state.clone());
        };

        let current_step = state.current_step();
        if current_step < slide.step_count {
            // Reveal next step within current slide
            state.update_step(current_step + 1);
        } else {
            // All steps revealed, go to first step of next slide
            let total_slides = self.total_slides().await;
            if let Some(next_slide_index) = state.next(total_slides) {
                state.update_slide(next_slide_index);
                state.update_step(0);
            }
        }

        Notification::state(state.clone())
    }

    async fn command_previous_step(&self, state: &mut State) -> Notification {
        let current_step = state.current_step();

        if current_step > 0 {
            // Just decrement step within current slide
            state.update_step(current_step - 1);
        } else {
            // At step 0, go to previous slide's last step
            let total_slides = self.total_slides().await;
            if let Some(prev_slide_index) = state.previous(total_slides)
                && let Some(prev_slide) = self.slide_by_index(prev_slide_index).await
            {
                state.update_slide(prev_slide_index);
                // Set to last step of previous slide (step_count means all steps revealed)
                state.update_step(prev_slide.step_count);
            }
        }

        Notification::state(state.clone())
    }

    pub async fn handle_command(&self, command: &Command) -> Notification {
        let start_time = std::time::Instant::now();
        let mut state = self.current_state.write().await;

        #[allow(clippy::match_same_arms)]
        let notification = match command {
            Command::Register { .. } => Notification::state(state.clone()),
            Command::Unregister { .. } => Notification::state(state.clone()),
            Command::First => self.command_first(&mut state).await,
            Command::Last => self.command_last(&mut state).await,
            Command::GoTo { slide } => self.command_goto(&mut state, *slide).await,
            Command::Next => self.command_next(&mut state).await,
            Command::Previous => self.command_previous(&mut state).await,
            Command::NextStep => self.command_next_step(&mut state).await,
            Command::PreviousStep => self.command_previous_step(&mut state).await,
            Command::Pause => Self::command_pause(&mut state),
            Command::Resume => Self::command_resume(&mut state),
            Command::Blink => Self::command_blink(),
            Command::Ping => Notification::PONG,
        };

        self.notify_all_clients(&notification);
        drop(state);

        tracing::debug!(
            ?command,
            duration_ms = start_time.elapsed().as_millis(),
            active_clients = self.clients.len(),
            "Command handled and broadcast completed"
        );

        notification
    }

    fn navigate_to_slide(state: &mut State, target_slide: usize, total_slides: usize) {
        match state {
            State::Init => {
                Self::transition_to_running(state, target_slide, false);
            }
            State::Paused { .. } => {
                if state.is_last_slide(total_slides) && target_slide == total_slides - 1 {
                    state.update_slide(target_slide);
                } else {
                    Self::transition_to_running(state, target_slide, false);
                }
            }
            _ => {
                state.update_slide(target_slide);
            }
        }
    }

    fn handle_next_in_running_state(state: &mut State, total_slides: usize) {
        if let Some(current) = state.current() {
            if let Some(next_slide) = state.next(total_slides) {
                state.update_slide(next_slide);
            } else if state.is_last_slide(total_slides) {
                let total_duration = state.calculate_total_duration();
                let current_step = state.current_step();
                *state = State::Done {
                    current,
                    current_step,
                    total_duration,
                };
            }
        } else {
            Self::transition_to_running(state, 0, false);
        }
    }

    fn has_capacity(&self) -> bool {
        self.clients.len() < self.max_clients
    }

    async fn create_initial_notification(&self) -> Notification {
        let current_state = self.current_state.read().await;
        Notification::state(current_state.clone())
    }

    fn notify_all_clients(&self, notification: &Notification) {
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

    pub async fn reload_talk(&self, new_talk: Talk) -> anyhow::Result<()> {
        if new_talk.slides.is_empty() {
            bail!("Cannot reload talk with empty slides");
        }

        let mut state = self.current_state.write().await;
        let current_slide_index = state.current().unwrap_or(0);

        let old_talk = self.talk.read().await;
        let current_slide = old_talk.slides.get(current_slide_index);

        // Preserve slide position: by title -> by index -> fallback to first
        let new_slide_index = Self::preserve_slide_position(
            current_slide,
            current_slide_index,
            &old_talk.slides,
            &new_talk.slides,
        );

        info!(
            old_slide = current_slide_index,
            new_slide = new_slide_index,
            old_title = ?current_slide.map(|slide| &slide.title),
            new_title = ?new_talk.slides.get(new_slide_index).map(|slide| &slide.title),
            "Talk reloaded"
        );

        // Update slide index in current state
        state.update_slide(new_slide_index);
        drop(old_talk);

        // Replace the talk
        let mut talk = self.talk.write().await;
        *talk = new_talk;
        drop(talk);

        // Send TalkChange notification to all clients
        let notification = Notification::talk_change(state.clone());
        self.notify_all_clients(&notification);

        Ok(())
    }

    fn preserve_slide_position(
        current_slide: Option<&Slide>,
        current_index: usize,
        old_slides: &[Slide],
        new_slides: &[Slide],
    ) -> usize {
        if let Some(slide) = current_slide {
            // Try to match by title (exact match first, then case-insensitive if text)
            if let Some(position) = new_slides
                .iter()
                .position(|new_slide| new_slide.title == slide.title)
            {
                return position;
            }

            // For text titles, try case-insensitive comparison
            if let Some(position) = Self::find_by_title_text(&slide.title, new_slides) {
                return position;
            }
        }

        // Try to preserve index if slide count unchanged
        if old_slides.len() == new_slides.len() && current_index < new_slides.len() {
            return current_index;
        }

        // Fallback to first slide
        0
    }

    fn find_by_title_text(title: &toboggan_core::Content, slides: &[Slide]) -> Option<usize> {
        use toboggan_core::Content;

        let title_text = match title {
            Content::Text { text } => text.to_lowercase(),
            Content::Html { alt: Some(alt), .. } => alt.to_lowercase(),
            Content::Html { raw, .. } => raw.to_lowercase(),
            _ => return None,
        };

        slides.iter().position(|slide| {
            let slide_text = match &slide.title {
                Content::Text { text } => text.to_lowercase(),
                Content::Html { alt: Some(alt), .. } => alt.to_lowercase(),
                Content::Html { raw, .. } => raw.to_lowercase(),
                _ => String::new(),
            };
            slide_text == title_text
        })
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
