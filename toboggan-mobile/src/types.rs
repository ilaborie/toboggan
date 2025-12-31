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
            title: title.clone(),
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
        let total_slides = slides.len();
        assert!(total_slides > 0, "total_slides must be greater than 0");

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
                    next: ((current_index as usize) < total_slides - 1).then(|| current_index + 1),
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
