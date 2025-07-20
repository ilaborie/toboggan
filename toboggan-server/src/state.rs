use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use jiff::Timestamp;
use toboggan_core::{Command, Notification, Slide, SlideId, State, Talk};

#[derive(Clone)]
pub struct TobogganState {
    pub(crate) started_at: Timestamp,
    pub(crate) talk: Arc<Talk>,
    pub(crate) slides: Arc<HashMap<SlideId, Slide>>,
    // Later capture client sender
    pub(crate) current_slide: SlideId,
    pub(crate) current_state: State,
    pub(crate) slide_order: Vec<SlideId>,
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

        Self {
            started_at: started,
            talk,
            slides,
            current_slide: first_slide,
            current_state,
            slide_order,
        }
    }

    pub fn handle_command(&mut self, command: &Command) -> Notification {
        match command {
            Command::Register { .. } | Command::Unregister { .. } => {
                // register/unregister will be handled later
            }
            Command::First => {
                if !self.current_state.is_first_slide(&self.slide_order) {
                    if let Some(&first_slide) = self.slide_order.first() {
                        self.current_slide = first_slide;
                        self.update_state_slide(first_slide);
                    }
                }
            }
            Command::Last => {
                if !self.current_state.is_last_slide(&self.slide_order) {
                    if let Some(&last_slide) = self.slide_order.last() {
                        self.current_slide = last_slide;
                        self.update_state_slide(last_slide);
                    }
                }
            }
            Command::GoTo(slide_id) => {
                if self.slide_order.contains(slide_id) {
                    self.current_slide = *slide_id;
                    self.update_state_slide(*slide_id);
                } else {
                    return Notification::error(format!("Slide with id {slide_id:?} not found"));
                }
            }
            Command::Next => {
                if let Some(next_slide) = self.current_state.next(&self.slide_order) {
                    self.current_slide = next_slide;
                    self.update_state_slide(next_slide);
                } else if self.current_state.is_last_slide(&self.slide_order) {
                    // We're at the last slide, mark as done
                    let total_duration = self.calculate_total_duration();
                    self.current_state = State::Done {
                        current: self.current_slide,
                        total_duration,
                    };
                }
            }
            Command::Previous => {
                if let Some(prev_slide) = self.current_state.previous(&self.slide_order) {
                    self.current_slide = prev_slide;
                    self.update_state_slide(prev_slide);
                }
            }
            Command::Pause => {
                // Skip if already paused or done
                if matches!(self.current_state, State::Running { .. }) {
                    let total_duration = self.calculate_total_duration();
                    self.current_state = State::Paused {
                        current: self.current_slide,
                        total_duration,
                    };
                }
            }
            Command::Resume => {
                // Skip if already running or done
                if matches!(self.current_state, State::Paused { .. }) {
                    self.auto_resume();
                }
            }
            Command::Ping => {
                return Notification::pong();
            }
        }

        Notification::state(self.current_state.clone())
    }

    fn auto_resume(&mut self) {
        let total_duration = self.calculate_total_duration();
        self.current_state = State::Running {
            since: Timestamp::now(),
            current: self.current_slide,
            total_duration,
        };
    }

    fn update_state_slide(&mut self, slide_id: SlideId) {
        let total_duration = self.calculate_total_duration();
        match &self.current_state {
            State::Running { .. } => {
                self.current_state = State::Running {
                    since: Timestamp::now(),
                    current: slide_id,
                    total_duration,
                };
            }
            State::Paused { .. } => {
                self.current_state = State::Paused {
                    current: slide_id,
                    total_duration,
                };
            }
            State::Done { .. } => {
                // When navigating from Done state, go back to Paused
                self.current_state = State::Paused {
                    current: slide_id,
                    total_duration,
                };
            }
        }
    }

    fn calculate_total_duration(&self) -> Duration {
        match &self.current_state {
            State::Paused { total_duration, .. } | State::Done { total_duration, .. } => {
                *total_duration
            }
            State::Running {
                since,
                total_duration,
                ..
            } => {
                let signed_duration = Timestamp::now().duration_since(*since);
                let current_duration =
                    TryInto::<Duration>::try_into(signed_duration).unwrap_or(Duration::ZERO);
                *total_duration + current_duration
            }
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
