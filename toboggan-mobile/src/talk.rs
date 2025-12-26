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
        current_step: u32,
        step_count: u32,
    },
    Paused {
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
    pub(crate) fn new(slides: &[super::Slide], value: &CoreState) -> Self {
        let total_slides = slides.len();
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
                current_step,
            } => {
                #[allow(clippy::cast_possible_truncation, clippy::expect_used)]
                // UniFFI requires u32, slide indices and step counts are typically small
                let current_index = current.expect("should have a current index").index() as u32;
                let step_count = slides
                    .get(current_index as usize)
                    .map_or(0, |slide| slide.step_count);
                #[allow(clippy::cast_possible_truncation)]
                Self::Paused {
                    previous: (current_index > 0).then(|| current_index - 1),
                    current: current_index,
                    next: ((current_index as usize) < total_slides - 1).then(|| current_index + 1),
                    current_step: current_step as u32,
                    step_count,
                }
            }
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
            Command::NextStep => Self::NextStep,
            Command::PreviousStep => Self::PreviousStep,
            Command::Resume => Self::Resume,
            Command::Pause => Self::Pause,
            Command::Blink => Self::Blink,
        }
    }
}
