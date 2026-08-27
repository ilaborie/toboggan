//! UniFFI-compatible type wrappers for toboggan-core types.
//!
//! These newtypes provide FFI-safe interfaces for Swift/Kotlin while
//! maintaining `From<CoreType>` implementations for easy conversion.

use toboggan_client::ConnectionStatus as CoreConnectionStatus;
use toboggan_core::{
    ClientRole as CoreClientRole, Command as CoreCommand, Slide as CoreSlide, SlideId,
    SlideKind as CoreSlideKind, State as CoreState, TalkResponse,
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
    /// Speaker notes, flattened to plain text.
    ///
    /// The phone is the device a presenter actually looks at mid-talk, so this
    /// is the one field that makes it more than a clicker. Empty when the slide
    /// has none, rather than `Option`, because "no notes" and "empty notes" read
    /// the same in a UI and an optional would only push the check to every caller.
    pub notes: String,
    /// Speaking time the author planned for this slide, in whole seconds.
    ///
    /// `None` when the deck plans nothing, which is what hides the pacing
    /// readout rather than showing it as zero.
    pub duration_secs: Option<u64>,
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
            // `display_text` is the projection built for exactly this: anything
            // that cannot show markup. The terminal client renders notes the
            // same way.
            notes: value.notes.display_text().to_owned(),
            duration_secs: value.duration.map(|planned| planned.as_secs()),
        }
    }
}

// ============================================================================
// State
// ============================================================================

/// Presentation state.
///
/// Named `PresentationState` rather than `State` because this type crosses into
/// `SwiftUI` and Compose, and both of those have a `State` of their own. Exported
/// as `State` it shadows theirs inside the host module, so the app cannot use
/// its own framework's property wrapper without qualifying every single use.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum PresentationState {
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

impl PresentationState {
    /// Create a new `PresentationState` from the core state and slides (for step counts)
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
    /// Jump straight to a slide, as the deck overview does.
    GoTo {
        slide: u32,
    },
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
            Command::GoTo { slide } => Self::GoTo {
                slide: SlideId::new(slide as usize),
            },
        }
    }
}

// ============================================================================
// ConnectionStatus
// ============================================================================

/// Connection status.
///
/// The payloads used to be dropped here, which left the app with "Reconnecting…"
/// and "Connection Error" and no way to say *which* attempt or *what* failed —
/// the two things anyone debugging a phone that will not reach the server needs.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Closed,
    Reconnecting {
        attempt: u32,
        max_attempt: u32,
        delay_secs: u64,
    },
    Error {
        message: String,
    },
}

impl From<CoreConnectionStatus> for ConnectionStatus {
    // `UniFFI` has no `usize`; a retry counter never approaches `u32`.
    #[allow(clippy::cast_possible_truncation)]
    fn from(value: CoreConnectionStatus) -> Self {
        match value {
            CoreConnectionStatus::Connecting => Self::Connecting,
            CoreConnectionStatus::Connected => Self::Connected,
            CoreConnectionStatus::Closed => Self::Closed,
            CoreConnectionStatus::Reconnecting {
                attempt,
                max_attempt,
                delay,
            } => Self::Reconnecting {
                attempt: attempt as u32,
                max_attempt: max_attempt as u32,
                delay_secs: delay.as_secs(),
            },
            CoreConnectionStatus::Error { message } => Self::Error { message },
        }
    }
}

/// The role the server granted this client.
///
/// Told, never asked for. A phone is never on the machine running the server, so
/// without a presenter token it is audience — and it has to say so rather than
/// let the user discover it by pressing a button that does nothing.
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum ClientRole {
    Presenter,
    Audience,
}

impl From<CoreClientRole> for ClientRole {
    fn from(value: CoreClientRole) -> Self {
        match value {
            CoreClientRole::Presenter => Self::Presenter,
            CoreClientRole::Audience => Self::Audience,
        }
    }
}

#[cfg(test)]
mod tests {
    use toboggan_core::{Content, SlideId, State as CoreState};

    use super::*;

    fn slide(title: &str) -> Slide {
        Slide {
            title: title.to_owned(),
            kind: SlideKind::Standard,
            step_count: 0,
            notes: String::new(),
            duration_secs: None,
        }
    }

    /// The client is legitimately in this state before the talk has loaded.
    /// It used to hit an `assert!` that aborted the host app across the FFI
    /// boundary, guarding a `total_slides - 1` that would otherwise underflow.
    /// The deck overview jumps straight to a slide, which the FFI could not
    /// express at all: `GoTo` is in the protocol but was not in this enum.
    #[test]
    fn goto_carries_the_slide_index_across_the_boundary() {
        match CoreCommand::from(Command::GoTo { slide: 7 }) {
            CoreCommand::GoTo { slide } => assert_eq!(slide.index(), 7),
            other => panic!("expected GoTo, got {other:?}"),
        }
    }

    /// Notes are the reason to look at a phone mid-talk, and they reach it as
    /// plain text: the phone has no HTML renderer.
    #[test]
    fn notes_cross_the_boundary_as_plain_text() {
        let core = CoreSlide {
            notes: Content::html_with_alt("<em>breathe</em>", "breathe"),
            ..CoreSlide::default()
        };
        let slide = Slide::from_core_slide(&core, 0);
        assert_eq!(slide.notes, "breathe");
    }

    /// A slide with no notes reads as empty rather than as absent, so no caller
    /// has to unwrap to render nothing.
    #[test]
    fn a_slide_without_notes_has_empty_notes() {
        let slide = Slide::from_core_slide(&CoreSlide::default(), 0);
        assert_eq!(slide.notes, "");
        assert_eq!(slide.duration_secs, None);
    }

    /// The payloads used to be dropped, leaving the app unable to say which
    /// attempt was failing or why.
    #[test]
    fn reconnecting_keeps_the_detail_the_app_needs_to_report() {
        let core = CoreConnectionStatus::Reconnecting {
            attempt: 3,
            max_attempt: 5,
            delay: std::time::Duration::from_secs(4),
        };
        match ConnectionStatus::from(core) {
            ConnectionStatus::Reconnecting {
                attempt,
                max_attempt,
                delay_secs,
            } => {
                assert_eq!((attempt, max_attempt, delay_secs), (3, 5, 4));
            }
            other => panic!("expected Reconnecting, got {other:?}"),
        }
    }

    #[test]
    fn an_error_keeps_its_message() {
        let core = CoreConnectionStatus::Error {
            message: "no route to host".to_owned(),
        };
        match ConnectionStatus::from(core) {
            ConnectionStatus::Error { message } => assert_eq!(message, "no route to host"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_deck_does_not_abort() {
        match PresentationState::new(&[], &CoreState::Init) {
            PresentationState::Init { total_slides } => assert_eq!(total_slides, 0),
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
        match PresentationState::new(&slides, &running) {
            PresentationState::Running { next, previous, .. } => {
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
        match PresentationState::new(&slides, &running) {
            PresentationState::Running { next, previous, .. } => {
                assert_eq!(previous, Some(0));
                assert_eq!(next, Some(2));
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }
}
