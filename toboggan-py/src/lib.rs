mod client_info;
mod slides;
mod state;
mod talk;
mod toboggan;

use pyo3::prelude::*;

pub use client_info::ClientInfo;
pub use slides::{Slide, Slides};
pub use state::State;
pub use talk::Talk;
pub use toboggan::Toboggan;

/// Toboggan for Python
#[pymodule]
fn toboggan_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Talk>()?;
    m.add_class::<Slides>()?;
    m.add_class::<Slide>()?;
    m.add_class::<State>()?;
    m.add_class::<ClientInfo>()?;
    m.add_class::<Toboggan>()?;

    Ok(())
}
