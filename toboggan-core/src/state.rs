use core::time::Duration;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::SlideId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum State {
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
    pub fn current(&self) -> SlideId {
        match self {
            Self::Paused { current, .. }
            | Self::Running { current, .. }
            | Self::Done { current, .. } => *current,
        }
    }

    #[cfg(feature = "std")]
    pub fn is_first_slide(&self, slide_order: &[SlideId]) -> bool {
        slide_order.first() == Some(&self.current())
    }

    #[cfg(feature = "std")]
    pub fn is_last_slide(&self, slide_order: &[SlideId]) -> bool {
        slide_order.last() == Some(&self.current())
    }

    #[cfg(feature = "std")]
    pub fn next(&self, slide_order: &[SlideId]) -> Option<SlideId> {
        let current = self.current();
        slide_order
            .iter()
            .position(|&id| id == current)
            .and_then(|pos| slide_order.get(pos + 1).copied())
    }

    #[cfg(feature = "std")]
    pub fn previous(&self, slide_order: &[SlideId]) -> Option<SlideId> {
        let current = self.current();
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
        let total_duration = self.calculate_total_duration();
        *self = Self::Running {
            since: Timestamp::now(),
            current: self.current(),
            total_duration,
        };
    }

    #[cfg(feature = "std")]
    pub fn update_slide(&mut self, slide_id: SlideId) {
        let total_duration = self.calculate_total_duration();
        match self {
            Self::Running { .. } => {
                *self = Self::Running {
                    since: Timestamp::now(),
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
    pub fn calculate_total_duration(&self) -> Duration {
        match self {
            Self::Paused { total_duration, .. } | Self::Done { total_duration, .. } => {
                *total_duration
            }
            Self::Running {
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
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    #[test]
    fn test_current() {
        let slide1 = SlideId::next();

        let state = State::Paused {
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.current(), slide1);

        let state = State::Running {
            since: Timestamp::now(),
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.current(), slide1);

        let state = State::Done {
            current: slide1,
            total_duration: Duration::from_secs(0),
        };
        assert_eq!(state.current(), slide1);
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
}
