use std::sync::Arc;

use anyhow::bail;
use toboggan_core::{Command, Content, Notification, Slide, SlideId, State, Talk};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Service for managing talk content and presentation state
#[derive(Clone)]
pub struct TalkService {
    talk: Arc<RwLock<Talk>>,
    current_state: Arc<RwLock<State>>,
}

impl TalkService {
    /// Creates a new `TalkService` with the given talk
    ///
    /// # Errors
    /// Returns an error if the talk has no slides
    pub fn new(talk: Talk) -> anyhow::Result<Self> {
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
            talk: Arc::new(RwLock::new(talk)),
            current_state,
        })
    }

    /// Returns the talk title
    pub async fn title(&self) -> String {
        let talk = self.talk.read().await;
        talk.title.clone()
    }

    /// Returns a clone of the talk
    pub async fn talk(&self) -> Talk {
        self.talk.read().await.clone()
    }

    /// Returns a clone of all slides
    pub async fn slides(&self) -> Vec<Slide> {
        let talk = self.talk.read().await;
        talk.slides.clone()
    }

    /// Returns a slide by its index
    pub async fn slide_by_index(&self, slide_id: SlideId) -> Option<Slide> {
        let talk = self.talk.read().await;
        talk.slides.get(slide_id.index()).cloned()
    }

    /// Returns the current presentation state
    pub async fn current_state(&self) -> State {
        let state = self.current_state.read().await;
        state.clone()
    }

    /// Creates an initial state notification for new clients
    pub async fn create_initial_notification(&self) -> Notification {
        let current_state = self.current_state.read().await;
        Notification::state(current_state.clone())
    }

    /// Handles a command and returns the notification (without broadcasting)
    pub async fn handle_command(&self, command: &Command) -> Notification {
        let mut state = self.current_state.write().await;

        match command {
            Command::Register { .. } | Command::Unregister { .. } => {
                Notification::state(state.clone())
            }
            Command::First => self.command_first(&mut state).await,
            Command::Last => self.command_last(&mut state).await,
            Command::GoTo { slide } => self.command_goto(&mut state, *slide).await,
            Command::NextSlide => self.command_next(&mut state).await,
            Command::PreviousSlide => self.command_previous(&mut state).await,
            Command::NextStep => self.command_next_step(&mut state).await,
            Command::PreviousStep => self.command_previous_step(&mut state).await,
            Command::Blink => Self::command_blink(),
            Command::Ping => Notification::PONG,
        }
    }

    /// Reloads talk and returns `TalkChange` notification (without broadcasting)
    ///
    /// # Errors
    /// Returns an error if the new talk has no slides
    pub async fn reload_talk(&self, new_talk: Talk) -> anyhow::Result<Notification> {
        if new_talk.slides.is_empty() {
            bail!("Cannot reload talk with empty slides");
        }

        let mut state = self.current_state.write().await;
        let current_slide_id = state.current().unwrap_or(SlideId::FIRST);

        let old_talk = self.talk.read().await;
        let current_slide = old_talk.slides.get(current_slide_id.index());

        // Preserve slide position: by title -> by index -> fallback to first
        let new_slide_id = Self::preserve_slide_position(
            current_slide,
            current_slide_id,
            &old_talk.slides,
            &new_talk.slides,
        );

        info!(
            old_slide = current_slide_id.index(),
            new_slide = new_slide_id.index(),
            old_title = ?current_slide.map(|slide| &slide.title),
            new_title = ?new_talk.slides.get(new_slide_id.index()).map(|slide| &slide.title),
            "Talk reloaded"
        );

        // Update slide index in current state
        state.update_slide(new_slide_id);
        drop(old_talk);

        // Replace the talk
        let mut talk = self.talk.write().await;
        *talk = new_talk;
        drop(talk);

        // Return TalkChange notification
        Ok(Notification::talk_change(state.clone()))
    }

    // === Private helper methods ===

    const NO_SLIDES_ERROR: &str = "No slides available";

    async fn total_slides(&self) -> usize {
        let talk = self.talk.read().await;
        talk.slides.len()
    }

    /// Returns total slides count, or None if empty (for early return pattern)
    async fn require_slides(&self) -> Option<usize> {
        let total = self.total_slides().await;
        if total == 0 { None } else { Some(total) }
    }

    fn transition_to_running(state: &mut State, slide_id: SlideId) {
        *state = State::Running {
            current: slide_id,
            current_step: 0,
        };
    }

    async fn command_first(&self, state: &mut State) -> Notification {
        let Some(total_slides) = self.require_slides().await else {
            return Notification::error(Self::NO_SLIDES_ERROR.to_string());
        };

        let should_transition = matches!(state, State::Init) || !state.is_first_slide(total_slides);

        if should_transition {
            Self::transition_to_running(state, SlideId::FIRST);
        }

        Notification::state(state.clone())
    }

    async fn command_last(&self, state: &mut State) -> Notification {
        let Some(total_slides) = self.require_slides().await else {
            return Notification::error(Self::NO_SLIDES_ERROR.to_string());
        };

        let last_slide = SlideId::new(total_slides - 1);
        Self::navigate_to_slide(state, last_slide);
        Notification::state(state.clone())
    }

    async fn command_goto(&self, state: &mut State, slide_id: SlideId) -> Notification {
        let total_slides = self.total_slides().await;
        if slide_id.index() >= total_slides {
            return Notification::error(format!(
                "Slide index {} not found, total slides: {total_slides}",
                slide_id.index()
            ));
        }

        Self::navigate_to_slide(state, slide_id);
        Notification::state(state.clone())
    }

    async fn command_next(&self, state: &mut State) -> Notification {
        let Some(total_slides) = self.require_slides().await else {
            warn!("{}", Self::NO_SLIDES_ERROR);
            return Notification::error(Self::NO_SLIDES_ERROR.to_string());
        };

        match state {
            State::Init => Self::transition_to_running(state, SlideId::FIRST),
            State::Running { .. } | State::Done { .. } => {
                Self::handle_next_in_running_state(state, total_slides);
            }
        }

        Notification::state(state.clone())
    }

    async fn command_previous(&self, state: &mut State) -> Notification {
        let Some(total_slides) = self.require_slides().await else {
            return Notification::error(Self::NO_SLIDES_ERROR.to_string());
        };

        match state {
            State::Init => Self::transition_to_running(state, SlideId::FIRST),
            State::Running { .. } | State::Done { .. } => {
                if let Some(prev_slide) = state.previous(total_slides) {
                    state.update_slide(prev_slide);
                }
            }
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

    fn navigate_to_slide(state: &mut State, target_slide: SlideId) {
        match state {
            State::Init => {
                Self::transition_to_running(state, target_slide);
            }
            State::Running { .. } | State::Done { .. } => {
                state.update_slide(target_slide);
            }
        }
    }

    fn handle_next_in_running_state(state: &mut State, total_slides: usize) {
        if let Some(current) = state.current() {
            if let Some(next_slide) = state.next(total_slides) {
                state.update_slide(next_slide);
            } else if state.is_last_slide(total_slides) {
                let current_step = state.current_step();
                *state = State::Done {
                    current,
                    current_step,
                };
            }
        } else {
            Self::transition_to_running(state, SlideId::FIRST);
        }
    }

    fn preserve_slide_position(
        current_slide: Option<&Slide>,
        current_id: SlideId,
        old_slides: &[Slide],
        new_slides: &[Slide],
    ) -> SlideId {
        if let Some(slide) = current_slide {
            // Try to match by title (exact match first, then case-insensitive if text)
            if let Some(position) = new_slides
                .iter()
                .position(|new_slide| new_slide.title == slide.title)
            {
                return SlideId::new(position);
            }

            // For text titles, try case-insensitive comparison
            if let Some(position) = Self::find_by_title_text(&slide.title, new_slides) {
                return SlideId::new(position);
            }
        }

        // Try to preserve index if slide count unchanged
        let current_index = current_id.index();
        if old_slides.len() == new_slides.len() && current_index < new_slides.len() {
            return current_id;
        }

        // Fallback to first slide
        SlideId::FIRST
    }

    fn find_by_title_text(title: &Content, slides: &[Slide]) -> Option<usize> {
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
