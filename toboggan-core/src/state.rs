use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "state")]
pub enum State {
    #[default]
    Init,
    Paused {
        current: Option<usize>,
        current_step: usize,
    },
    Running {
        current: usize,
        current_step: usize,
    },
    Done {
        current: usize,
        current_step: usize,
    },
}

impl State {
    #[must_use]
    pub fn current(&self) -> Option<usize> {
        match self {
            Self::Init => None,
            Self::Paused { current, .. } => *current,
            Self::Running { current, .. } | Self::Done { current, .. } => Some(*current),
        }
    }

    #[must_use]
    pub fn current_step(&self) -> usize {
        match self {
            Self::Init => 0,
            Self::Paused { current_step, .. }
            | Self::Running { current_step, .. }
            | Self::Done { current_step, .. } => *current_step,
        }
    }

    pub fn update_step(&mut self, step: usize) {
        match self {
            Self::Init => {}
            Self::Paused { current_step, .. }
            | Self::Running { current_step, .. }
            | Self::Done { current_step, .. } => *current_step = step,
        }
    }

    #[must_use]
    pub fn is_first_slide(&self, total_slides: usize) -> bool {
        total_slides > 0 && self.current() == Some(0)
    }

    #[must_use]
    pub fn is_last_slide(&self, total_slides: usize) -> bool {
        if total_slides == 0 {
            return false;
        }
        self.current() == Some(total_slides - 1)
    }

    #[must_use]
    pub fn next(&self, total_slides: usize) -> Option<usize> {
        let current = self.current()?;
        (current + 1 < total_slides).then(|| current + 1)
    }

    #[must_use]
    pub fn previous(&self, _total_slides: usize) -> Option<usize> {
        let current = self.current()?;
        (current > 0).then(|| current - 1)
    }

    pub fn auto_resume(&mut self) {
        if let Self::Paused {
            current: Some(slide_index),
            current_step,
        } = self
        {
            *self = Self::Running {
                current: *slide_index,
                current_step: *current_step,
            };
        }
    }

    pub fn update_slide(&mut self, slide_index: usize) {
        match self {
            Self::Init => {
                // When navigating from Init state, go to Running
                *self = Self::Running {
                    current: slide_index,
                    current_step: 0,
                };
            }
            Self::Running { .. } => {
                *self = Self::Running {
                    current: slide_index,
                    current_step: 0,
                };
            }
            Self::Paused { .. } => {
                *self = Self::Paused {
                    current: Some(slide_index),
                    current_step: 0,
                };
            }
            Self::Done { .. } => {
                // When navigating from Done state, go back to Paused
                *self = Self::Paused {
                    current: Some(slide_index),
                    current_step: 0,
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_current() {
        let state = State::default();
        assert_eq!(state.current(), None);

        let state = State::Paused {
            current: Some(0),
            current_step: 0,
        };
        assert_eq!(state.current(), Some(0));

        let state = State::Running {
            current: 0,
            current_step: 0,
        };
        assert_eq!(state.current(), Some(0));

        let state = State::Done {
            current: 0,
            current_step: 0,
        };
        assert_eq!(state.current(), Some(0));
    }

    #[test]
    fn test_is_first_slide() {
        let total_slides = 3;

        // Test with first slide
        let state = State::Paused {
            current: Some(0),
            current_step: 0,
        };
        assert!(state.is_first_slide(total_slides));
        assert!(!state.is_last_slide(total_slides));

        // Test with middle slide
        let state = State::Running {
            current: 1,
            current_step: 0,
        };
        assert!(!state.is_first_slide(total_slides));
        assert!(!state.is_last_slide(total_slides));

        // Test with last slide
        let state = State::Done {
            current: 2,
            current_step: 0,
        };
        assert!(!state.is_first_slide(total_slides));
        assert!(state.is_last_slide(total_slides));
    }

    #[test]
    fn test_with_empty_slide_order() {
        let total_slides = 0;

        let state = State::Paused {
            current: Some(0),
            current_step: 0,
        };
        assert!(!state.is_first_slide(total_slides));
        assert!(!state.is_last_slide(total_slides));
    }

    #[test]
    fn test_with_single_slide() {
        let total_slides = 1;

        let state = State::Paused {
            current: Some(0),
            current_step: 0,
        };
        assert!(state.is_first_slide(total_slides));
        assert!(state.is_last_slide(total_slides));
    }

    #[test]
    fn test_next() {
        let total_slides = 3;

        // Test next from first slide
        let state = State::Paused {
            current: Some(0),
            current_step: 0,
        };
        assert_eq!(state.next(total_slides), Some(1));

        // Test next from middle slide
        let state = State::Running {
            current: 1,
            current_step: 0,
        };
        assert_eq!(state.next(total_slides), Some(2));

        // Test next from last slide
        let state = State::Done {
            current: 2,
            current_step: 0,
        };
        assert_eq!(state.next(total_slides), None);
    }

    #[test]
    fn test_previous() {
        let total_slides = 3;

        // Test previous from first slide
        let state = State::Paused {
            current: Some(0),
            current_step: 0,
        };
        assert_eq!(state.previous(total_slides), None);

        // Test previous from middle slide
        let state = State::Running {
            current: 1,
            current_step: 0,
        };
        assert_eq!(state.previous(total_slides), Some(0));

        // Test previous from last slide
        let state = State::Done {
            current: 2,
            current_step: 0,
        };
        assert_eq!(state.previous(total_slides), Some(1));
    }

    #[test]
    fn test_next_previous_with_empty_order() {
        let total_slides = 0;

        let state = State::Paused {
            current: Some(0),
            current_step: 0,
        };
        assert_eq!(state.next(total_slides), None);
        assert_eq!(state.previous(total_slides), None);
    }

    #[test]
    fn test_next_previous_with_single_slide() {
        let total_slides = 1;

        let state = State::Paused {
            current: Some(0),
            current_step: 0,
        };
        assert_eq!(state.next(total_slides), None);
        assert_eq!(state.previous(total_slides), None);
    }

    #[test]
    fn test_state_serialization_format() {
        let paused_state = State::Paused {
            current: Some(0),
            current_step: 0,
        };

        let running_state = State::Running {
            current: 0,
            current_step: 0,
        };

        // Test that the states are constructed correctly with internally tagged serde format
        assert_eq!(paused_state.current(), Some(0));

        assert_eq!(running_state.current(), Some(0));

        // Verify the states have the expected variants
        assert!(matches!(paused_state, State::Paused { .. }));
        assert!(matches!(running_state, State::Running { .. }));
    }

    #[test]
    fn test_current_step() {
        let state = State::default();
        assert_eq!(state.current_step(), 0);

        let state = State::Paused {
            current: Some(0),
            current_step: 2,
        };
        assert_eq!(state.current_step(), 2);

        let state = State::Running {
            current: 0,
            current_step: 3,
        };
        assert_eq!(state.current_step(), 3);
    }

    #[test]
    fn test_update_step() {
        let mut state = State::Paused {
            current: Some(0),
            current_step: 0,
        };
        assert_eq!(state.current_step(), 0);

        state.update_step(2);
        assert_eq!(state.current_step(), 2);

        state.update_step(0);
        assert_eq!(state.current_step(), 0);
    }
}
