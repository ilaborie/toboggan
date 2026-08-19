//! UniFFI-compatible type wrappers for toboggan-core types.
//!
//! These newtypes provide FFI-safe interfaces for Swift/Kotlin while
//! maintaining `From<CoreType>` implementations for easy conversion.

use toboggan_client::ConnectionStatus as CoreConnectionStatus;
use toboggan_core::{
    Command as CoreCommand, Slide as CoreSlide, SlideKind as CoreSlideKind, State as CoreState,
    TalkResponse,
};

// ============================================================================
// Talk
// ============================================================================

/// A talk (presentation metadata)
#[derive(Debug, Clone, uniffi::Record)]
pub struct Talk {
    pub title: String,
    pub date: String,
    pub slides: Vec<String>,
}

impl From<TalkResponse> for Talk {
    fn from(value: TalkResponse) -> Self {
        let TalkResponse {
            title,
            date,
            titles,
            ..
        } = value;

        Self {
            title,
            date: date.to_string(),
            slides: titles,
        }
    }
}

// ============================================================================
// Slide
// ============================================================================

/// A slide kind
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum SlideKind {
    /// Cover slide
    Cover,
    /// Part header slide
    Part,
    /// Standard slide
    Standard,
}

/// A slide
#[derive(Debug, Clone, uniffi::Record)]
pub struct Slide {
    pub title: String,
    pub kind: SlideKind,
    pub step_count: u32,
}

impl Slide {
    /// Create a Slide from a `CoreSlide` with step count from server.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_core_slide(value: &CoreSlide, step_count: usize) -> Self {
        // UniFFI requires u32, step counts are typically small
        Self {
            title: value.title.to_string(),
            kind: match value.kind {
                CoreSlideKind::Cover => SlideKind::Cover,
                CoreSlideKind::Part => SlideKind::Part,
                CoreSlideKind::Standard => SlideKind::Standard,
            },
            step_count: step_count as u32,
        }
    }
}

// ============================================================================
// State
// ============================================================================

/// Presentation state
#[derive(Debug, Clone, uniffi::Enum)]
pub enum State {
    Init {
        total_slides: u32,
    },
    Running {
        previous: Option<u32>,
        current: u32,
        next: Option<u32>,
        current_step: u32,
        step_count: u32,
    },
    Done {
        previous: Option<u32>,
        current: u32,
        current_step: u32,
        step_count: u32,
    },
}

impl State {
    /// Create a new State from core State and slides (for step count calculation)
    pub(crate) fn new(slides: &[Slide], value: &CoreState) -> Self {
        // No assertion that the deck is non-empty. There used to be one, because
        // the `next` calculation below computed `total_slides - 1` and would
        // underflow — but an empty deck is a state the client is legitimately in
        // before the talk has loaded, and an `assert!` across the UniFFI boundary
        // aborts the host app. The subtraction is gone instead.
        let total_slides = slides.len();

        #[allow(clippy::cast_possible_truncation)]
        // UniFFI requires u32, truncation unlikely for slide counts
        let total_slides_u32 = total_slides as u32;

        match *value {
            CoreState::Init => Self::Init {
                total_slides: total_slides_u32,
            },
            CoreState::Running {
                current,
                current_step,
            } => {
                #[allow(clippy::cast_possible_truncation)]
                // UniFFI requires u32, slide indices and step counts are typically small
                let current_index = current.index() as u32;
                let step_count = slides
                    .get(current_index as usize)
                    .map_or(0, |slide| slide.step_count);
                #[allow(clippy::cast_possible_truncation)]
                Self::Running {
                    previous: (current_index > 0).then(|| current_index - 1),
                    current: current_index,
                    next: ((current_index as usize + 1) < total_slides).then(|| current_index + 1),
                    current_step: current_step as u32,
                    step_count,
                }
            }
            CoreState::Done {
                current,
                current_step,
            } => {
                #[allow(clippy::cast_possible_truncation)]
                // UniFFI requires u32, slide indices and step counts are typically small
                let current_index = current.index() as u32;
                let step_count = slides
                    .get(current_index as usize)
                    .map_or(0, |slide| slide.step_count);
                #[allow(clippy::cast_possible_truncation)]
                Self::Done {
                    previous: (current_index > 0).then(|| current_index - 1),
                    current: current_index,
                    current_step: current_step as u32,
                    step_count,
                }
            }
        }
    }
}

// ============================================================================
// Command
// ============================================================================

/// Commands that can be sent to the server
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum Command {
    // Slide navigation
    Next,
    Previous,
    First,
    Last,
    // Step navigation
    NextStep,
    PreviousStep,
    // Presentation control
    Blink,
}

impl From<Command> for CoreCommand {
    fn from(value: Command) -> Self {
        match value {
            Command::Next => Self::NextSlide,
            Command::Previous => Self::PreviousSlide,
            Command::First => Self::First,
            Command::Last => Self::Last,
            Command::NextStep => Self::NextStep,
            Command::PreviousStep => Self::PreviousStep,
            Command::Blink => Self::Blink,
        }
    }
}

// ============================================================================
// ConnectionStatus
// ============================================================================

/// Connection status (simplified for `UniFFI`)
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Closed,
    Reconnecting,
    Error,
}

impl From<CoreConnectionStatus> for ConnectionStatus {
    fn from(value: CoreConnectionStatus) -> Self {
        match value {
            CoreConnectionStatus::Connecting => Self::Connecting,
            CoreConnectionStatus::Connected => Self::Connected,
            CoreConnectionStatus::Closed => Self::Closed,
            CoreConnectionStatus::Reconnecting { .. } => Self::Reconnecting,
            CoreConnectionStatus::Error { .. } => Self::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use toboggan_core::{SlideId, State as CoreState};

    use super::*;

    fn slide(title: &str) -> Slide {
        Slide {
            title: title.to_owned(),
            kind: SlideKind::Standard,
            step_count: 0,
        }
    }

    /// The client is legitimately in this state before the talk has loaded.
    /// It used to hit an `assert!` that aborted the host app across the FFI
    /// boundary, guarding a `total_slides - 1` that would otherwise underflow.
    #[test]
    fn an_empty_deck_does_not_abort() {
        match State::new(&[], &CoreState::Init) {
            State::Init { total_slides } => assert_eq!(total_slides, 0),
            other => panic!("expected Init, got {other:?}"),
        }
    }

    #[test]
    fn the_last_slide_has_no_next() {
        let slides = [slide("one"), slide("two")];
        let running = CoreState::Running {
            current: SlideId::new(1),
            current_step: 0,
        };
        match State::new(&slides, &running) {
            State::Running { next, previous, .. } => {
                assert_eq!(next, None, "last slide has no next");
                assert_eq!(previous, Some(0));
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn a_middle_slide_has_both_neighbours() {
        let slides = [slide("one"), slide("two"), slide("three")];
        let running = CoreState::Running {
            current: SlideId::new(1),
            current_step: 0,
        };
        match State::new(&slides, &running) {
            State::Running { next, previous, .. } => {
                assert_eq!(previous, Some(0));
                assert_eq!(next, Some(2));
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }
}
