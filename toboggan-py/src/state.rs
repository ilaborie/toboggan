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
        self.0.current().map(|current| current.index() + 1)
    }

    /// The current step within the slide, or None if not started.
    #[getter]
    fn step(&self) -> Option<usize> {
        self.0.current().map(|_| self.0.current_step())
    }

    /// Whether the deck is on its first slide.
    ///
    /// Takes the deck's slide count, like its counterpart: an empty deck is on
    /// neither its first nor its last slide, and that is decided once, in
    /// `toboggan-core`, rather than again here.
    fn is_first_slide(&self, total_slides: usize) -> bool {
        self.0.is_first_slide(total_slides)
    }

    /// Whether the deck is on its last slide.
    fn is_last_slide(&self, total_slides: usize) -> bool {
        self.0.is_last_slide(total_slides)
    }
}
