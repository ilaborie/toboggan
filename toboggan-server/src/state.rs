use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use jiff::Timestamp;
use serde_json::json;
use toboggan_core::{Command, Notification, Slide, SlideId, State, Talk};
use tokio::sync::RwLock;
#[derive(Clone)]
pub struct TobogganState {
    started_at: Timestamp,
    talk: Arc<Talk>,
    slides: Arc<HashMap<SlideId, Slide>>,
    slide_order: Arc<[SlideId]>,
    // Later capture client sender
    current_state: Arc<RwLock<State>>,
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

    pub async fn handle_command(&self, command: &Command) -> Notification {
        let mut state = self.current_state.write().await;
        match command {
            Command::Register { .. } | Command::Unregister { .. } => {
                // register/unregister will be handled later
            }
            Command::First => {
                if !state.is_first_slide(&self.slide_order) {
                    if let Some(&first_slide) = self.slide_order.first() {
                        state.update_slide(first_slide);
                    }
                }
            }
            Command::Last => {
                if !state.is_last_slide(&self.slide_order) {
                    if let Some(&last_slide) = self.slide_order.last() {
                        state.update_slide(last_slide);
                    }
                }
            }
            Command::GoTo(slide_id) => {
                if self.slide_order.contains(slide_id) {
                    state.update_slide(*slide_id);
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
            }
            Command::Previous => {
                if let Some(prev_slide) = state.previous(&self.slide_order) {
                    state.update_slide(prev_slide);
                }
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
            }
            Command::Resume => {
                // Skip if already running or done
                if matches!(*state, State::Paused { .. }) {
                    state.auto_resume();
                }
            }
            Command::Ping => {
                return Notification::pong();
            }
        }

        Notification::state(state.clone())
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
