use std::time::Duration;

use toboggan_core::{Command as CoreCommand, State as CoreState, TalkResponse};

/// A talk
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

#[derive(Debug, Clone, uniffi::Enum)]
pub enum State {
    Init {
        total_slides: u32,
    },
    Running {
        previous: Option<u32>,
        current: u32,
        next: Option<u32>,
        total_duration: Duration,
    },
    Paused {
        previous: Option<u32>,
        current: u32,
        next: Option<u32>,
        total_duration: Duration,
    },
    Done {
        previous: Option<u32>,
        current: u32,
        total_duration: Duration,
    },
}

impl State {
    pub(crate) fn new(total_slides: usize, value: &CoreState) -> Self {
        assert!(total_slides > 0, "total_slides must be greater than 0");
        #[allow(clippy::cast_possible_truncation)]
        // UniFFI requires u32, truncation unlikely for slide counts
        let total_slides_u32 = total_slides as u32;

        match *value {
            CoreState::Init => Self::Init {
                total_slides: total_slides_u32,
            },
            CoreState::Paused {
                current,
                total_duration,
                ..
            } => {
                #[allow(clippy::cast_possible_truncation, clippy::expect_used)]
                // UniFFI requires u32, slide indices are typically small
                let current_index = current.expect("should have a current index") as u32;
                Self::Paused {
                    previous: (current_index > 0).then(|| current_index - 1),
                    current: current_index,
                    next: ((current_index as usize) < total_slides - 1).then(|| current_index + 1),
                    total_duration: total_duration.into(),
                }
            }
            CoreState::Running {
                current,
                total_duration,
                ..
            } => {
                #[allow(clippy::cast_possible_truncation)]
                // UniFFI requires u32, slide indices are typically small
                let current_index = current as u32;
                Self::Running {
                    previous: (current_index > 0).then(|| current_index - 1),
                    current: current_index,
                    next: ((current_index as usize) < total_slides - 1).then(|| current_index + 1),
                    total_duration: total_duration.into(),
                }
            }
            CoreState::Done {
                current,
                total_duration,
                ..
            } => {
                #[allow(clippy::cast_possible_truncation)]
                // UniFFI requires u32, slide indices are typically small
                let current_index = current as u32;
                Self::Done {
                    previous: (current_index > 0).then(|| current_index - 1),
                    current: current_index,
                    total_duration: total_duration.into(),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum Command {
    Next,
    Previous,
    First,
    Last,
    Pause,
    Resume,
    Blink,
}

impl From<Command> for CoreCommand {
    fn from(value: Command) -> Self {
        match value {
            Command::Next => Self::Next,
            Command::Previous => Self::Previous,
            Command::First => Self::First,
            Command::Last => Self::Last,
            Command::Resume => Self::Resume,
            Command::Pause => Self::Pause,
            Command::Blink => Self::Blink,
        }
    }
}
