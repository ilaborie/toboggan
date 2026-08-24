use pyo3::prelude::*;
use toboggan_core::State as TState;

/// Current presentation state.
///
/// Carries the deck's slide count alongside the state itself. `is_first_slide`
/// and `is_last_slide` used to take it as an argument, which made
/// `state.is_first_slide(999)` a well-typed lie and left the caller stitching
/// together two separate reads — `state.is_last_slide(len(client.slides))` —
/// that a deck reload landing between them could take from different decks.
/// The one object that knows the true count is the client that mints this, so
/// it supplies it.
#[pyclass]
pub struct State(pub(crate) TState, pub(crate) usize);

#[pymethods]
impl State {
    fn __repr__(&self) -> String {
        match &self.0 {
            TState::Init => "State(Init)".to_owned(),
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

    /// Which of the three states this is: `"init"`, `"running"` or `"done"`.
    ///
    /// The booleans above are the same question asked three times, and nothing
    /// in their types says exactly one of them is true. This is the underlying
    /// sum, so a caller can match on it and a type-checker can narrow it.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.0 {
            TState::Init => "init",
            TState::Running { .. } => "running",
            TState::Done { .. } => "done",
        }
    }

    /// Whether the deck is on its first slide.
    ///
    /// An empty deck is on neither its first nor its last slide, and that is
    /// decided once, in `toboggan-core`, rather than again here.
    #[getter]
    fn is_first_slide(&self) -> bool {
        self.0.is_first_slide(self.1)
    }

    /// Whether the deck is on its last slide.
    #[getter]
    fn is_last_slide(&self) -> bool {
        self.0.is_last_slide(self.1)
    }

    /// How many slides the deck had when this state was read.
    #[getter]
    fn total_slides(&self) -> usize {
        self.1
    }
}
