use pyo3::prelude::*;
use toboggan_core::State as TState;

/// Current presentation state.
#[pyclass]
pub struct State(pub(crate) TState);

#[pymethods]
impl State {
    fn __repr__(&self) -> String {
        match &self.0 {
            TState::Init => "State(Init)".to_string(),
            TState::Running {
                current,
                current_step,
            } => format!("State(Running, slide: {current}, step: {current_step})"),
            TState::Done {
                current,
                current_step,
            } => format!("State(Done, slide: {current}, step: {current_step})"),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Whether the presentation is in the initial state.
    #[getter]
    fn is_init(&self) -> bool {
        matches!(self.0, TState::Init)
    }

    /// Whether the presentation is currently running.
    #[getter]
    fn is_running(&self) -> bool {
        matches!(self.0, TState::Running { .. })
    }

    /// Whether the presentation is finished.
    #[getter]
    fn is_done(&self) -> bool {
        matches!(self.0, TState::Done { .. })
    }

    /// The current slide number (1-indexed), or None if not started.
    #[getter]
    fn slide(&self) -> Option<usize> {
        match &self.0 {
            TState::Init => None,
            TState::Running { current, .. } | TState::Done { current, .. } => {
                Some(current.index() + 1)
            }
        }
    }

    /// The current step within the slide, or None if not started.
    #[getter]
    fn step(&self) -> Option<usize> {
        match &self.0 {
            TState::Init => None,
            TState::Running { current_step, .. } | TState::Done { current_step, .. } => {
                Some(*current_step)
            }
        }
    }

    /// Check if currently on the first slide.
    pub fn is_first_slide(&self) -> bool {
        match &self.0 {
            TState::Init => false,
            TState::Running { current, .. } | TState::Done { current, .. } => current.index() == 0,
        }
    }

    /// Check if currently on the last slide (requires total slide count).
    pub fn is_last_slide(&self, total_slides: usize) -> bool {
        match &self.0 {
            TState::Init => false,
            TState::Running { current, .. } | TState::Done { current, .. } => {
                current.index() + 1 >= total_slides
            }
        }
    }
}
