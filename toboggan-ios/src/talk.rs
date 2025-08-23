use std::time::Duration;

use toboggan_core::{Command as CoreCommand, State as CoreState, TalkResponse};

use crate::Id;

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
            title: title.to_string(),
            date: date.to_string(),
            slides: titles,
        }
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum State {
    Init {
        next: Id,
    },
    Running {
        previous: Option<Id>,
        current: Id,
        next: Option<Id>,
        total_duration: Duration,
    },
    Paused {
        previous: Option<Id>,
        current: Id,
        next: Option<Id>,
        total_duration: Duration,
    },
    Done {
        previous: Option<Id>,
        current: Id,
        total_duration: Duration,
    },
}

impl State {
    pub(crate) fn new(ids: &[Id], value: &CoreState) -> Self {
        assert!(!ids.is_empty());
        match *value {
            CoreState::Init => {
                let next = ids.first().cloned().expect("ids not empty");
                Self::Init { next }
            }
            CoreState::Paused {
                current,
                total_duration,
            } => {
                let current = current.expect("should have a current id").to_string();
                let index = ids
                    .iter()
                    .position(|it| it == &current)
                    .expect("should have the current id");
                Self::Paused {
                    previous: if index > 0 {
                        ids.get(index - 1).cloned()
                    } else {
                        None
                    },
                    current,
                    next: ids.get(index + 1).cloned(),
                    total_duration: total_duration.into(),
                }
            }
            CoreState::Running {
                current,
                total_duration,
                ..
            } => {
                let current = current.to_string();
                let index = ids
                    .iter()
                    .position(|it| it == &current)
                    .expect("should have the current id");
                Self::Running {
                    previous: if index > 0 {
                        ids.get(index - 1).cloned()
                    } else {
                        None
                    },
                    current,
                    next: ids.get(index + 1).cloned(),
                    total_duration: total_duration.into(),
                }
            }
            CoreState::Done {
                current,
                total_duration,
            } => {
                let current = current.to_string();
                let index = ids
                    .iter()
                    .position(|it| it == &current)
                    .expect("should have the current id");
                Self::Done {
                    previous: if index > 0 {
                        ids.get(index - 1).cloned()
                    } else {
                        None
                    },
                    current,
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
