use core::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{SlideId, Timestamp};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum State {
    #[default]
    Init,
    Paused {
        current: SlideId,
        total_duration: Duration,
    },
    Running {
        since: Timestamp,
        current: SlideId,
        total_duration: Duration,
    },
    Done {
        current: SlideId,
        total_duration: Duration,
    },
}

impl State {
    #[must_use]
    pub fn current(&self) -> Option<SlideId> {
        let result = match self {
            Self::Init => {
                return None;
            }
            Self::Paused { current, .. }
            | Self::Running { current, .. }
            | Self::Done { current, .. } => *current,
        };
        Some(result)
    }

    #[cfg(feature = "std")]
    #[must_use]
    pub fn is_first_slide(&self, slide_order: &[SlideId]) -> bool {
        slide_order.first() == self.current().as_ref()
    }

    #[cfg(feature = "std")]
    #[must_use]
    pub fn is_last_slide(&self, slide_order: &[SlideId]) -> bool {
        slide_order.last() == self.current().as_ref()
    }

    #[cfg(feature = "std")]
    #[must_use]
    pub fn next(&self, slide_order: &[SlideId]) -> Option<SlideId> {
        let current = self.current()?;
        slide_order
            .iter()
            .position(|&id| id == current)
            .and_then(|pos| slide_order.get(pos + 1).copied())
    }

    #[cfg(feature = "std")]
    #[must_use]
    pub fn previous(&self, slide_order: &[SlideId]) -> Option<SlideId> {
        let current = self.current()?;
        slide_order
            .iter()
            .position(|&id| id == current)
            .and_then(|pos| {
                if pos > 0 {
                    slide_order.get(pos - 1).copied()
                } else {
                    None
                }
            })
    }

    #[cfg(feature = "std")]
    pub fn auto_resume(&mut self) {
        let Some(current) = self.current() else {
            return;
        };
        let total_duration = self.calculate_total_duration();
        *self = Self::Running {
            since: Timestamp::now(),
            current,
            total_duration,
        };
    }

    #[cfg(feature = "std")]
    pub fn update_slide(&mut self, slide_id: SlideId) {
        let total_duration = self.calculate_total_duration();
        match self {
            Self::Init => {
                *self = Self::Running {
                    since: Timestamp::now(),
                    current: slide_id,
                    total_duration,
                };
            }
            Self::Running { since, .. } => {
                *self = Self::Running {
                    since: *since,
                    current: slide_id,
                    total_duration,
                };
            }
            Self::Paused { .. } => {
                *self = Self::Paused {
                    current: slide_id,
                    total_duration,
                };
            }
            Self::Done { .. } => {
                // When navigating from Done state, go back to Paused
                *self = Self::Paused {
                    current: slide_id,
                    total_duration,
                };
            }
        }
    }

    #[cfg(feature = "std")]
    #[must_use]
    pub fn calculate_total_duration(&self) -> Duration {
        match self {
            Self::Init => Duration::ZERO,
            Self::Paused { total_duration, .. } | Self::Done { total_duration, .. } => {
                *total_duration
            }
            Self::Running {
                since,
                total_duration,
                ..
            } => *total_duration + since.elapsed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;

    #[test]
    fn test_current() {
        let state = State::default();
        assert_eq!(state.current(), None);

        let slide1 = SlideId::next();

        let state = State::Paused {
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.current(), Some(slide1));

        let state = State::Running {
            since: Timestamp::now(),
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.current(), Some(slide1));

        let state = State::Done {
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.current(), Some(slide1));
    }

    #[test]
    fn test_is_first_slide() {
        let slide1 = SlideId::next();
        let slide2 = SlideId::next();
        let slide3 = SlideId::next();
        let slide_order = vec![slide1, slide2, slide3];

        // Test with first slide
        let state = State::Paused {
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert!(state.is_first_slide(&slide_order));
        assert!(!state.is_last_slide(&slide_order));

        // Test with middle slide
        let state = State::Running {
            since: Timestamp::now(),
            current: slide2,
            total_duration: Duration::from_secs(0),
        };
        assert!(!state.is_first_slide(&slide_order));
        assert!(!state.is_last_slide(&slide_order));

        // Test with last slide
        let state = State::Done {
            current: slide3,
            total_duration: Duration::from_secs(0),
        };
        assert!(!state.is_first_slide(&slide_order));
        assert!(state.is_last_slide(&slide_order));
    }

    #[test]
    fn test_with_empty_slide_order() {
        let slide1 = SlideId::next();
        let empty_order: Vec<SlideId> = vec![];

        let state = State::Paused {
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert!(!state.is_first_slide(&empty_order));
        assert!(!state.is_last_slide(&empty_order));
    }

    #[test]
    fn test_with_single_slide() {
        let slide1 = SlideId::next();
        let slide_order = vec![slide1];

        let state = State::Paused {
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert!(state.is_first_slide(&slide_order));
        assert!(state.is_last_slide(&slide_order));
    }

    #[test]
    fn test_next() {
        let slide1 = SlideId::next();
        let slide2 = SlideId::next();
        let slide3 = SlideId::next();
        let slide_order = vec![slide1, slide2, slide3];

        // Test next from first slide
        let state = State::Paused {
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.next(&slide_order), Some(slide2));

        // Test next from middle slide
        let state = State::Running {
            since: Timestamp::now(),
            current: slide2,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.next(&slide_order), Some(slide3));

        // Test next from last slide
        let state = State::Done {
            current: slide3,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.next(&slide_order), None);
    }

    #[test]
    fn test_previous() {
        let slide1 = SlideId::next();
        let slide2 = SlideId::next();
        let slide3 = SlideId::next();
        let slide_order = vec![slide1, slide2, slide3];

        // Test previous from first slide
        let state = State::Paused {
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.previous(&slide_order), None);

        // Test previous from middle slide
        let state = State::Running {
            since: Timestamp::now(),
            current: slide2,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.previous(&slide_order), Some(slide1));

        // Test previous from last slide
        let state = State::Done {
            current: slide3,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.previous(&slide_order), Some(slide2));
    }

    #[test]
    fn test_next_previous_with_empty_order() {
        let slide1 = SlideId::next();
        let empty_order: Vec<SlideId> = vec![];

        let state = State::Paused {
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.next(&empty_order), None);
        assert_eq!(state.previous(&empty_order), None);
    }

    #[test]
    fn test_next_previous_with_single_slide() {
        let slide1 = SlideId::next();
        let slide_order = vec![slide1];

        let state = State::Paused {
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.next(&slide_order), None);
        assert_eq!(state.previous(&slide_order), None);
    }

    #[test]
    fn test_state_serialization_format() {
        let slide_id = SlideId::next();

        let paused_state = State::Paused {
            current: slide_id,
            total_duration: Duration::from_secs(10),
        };

        let running_state = State::Running {
            since: Timestamp::now(),
            current: slide_id,
            total_duration: Duration::from_secs(5),
        };

        // Test that the states are constructed correctly with internally tagged serde format
        assert_eq!(paused_state.current(), Some(slide_id));

        assert_eq!(running_state.current(), Some(slide_id));

        // Verify the states have the expected variants
        assert!(matches!(paused_state, State::Paused { .. }));
        assert!(matches!(running_state, State::Running { .. }));
    }
}
