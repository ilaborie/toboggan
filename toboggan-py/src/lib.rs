mod client_info;
mod slides;
mod state;
mod talk;
mod toboggan;

pub use client_info::ClientInfo;
use pyo3::prelude::*;
pub use slides::{Slide, Slides, SlidesIter};
pub use state::State;
pub use talk::Talk;
pub use toboggan::Toboggan;

/// Toboggan for Python
#[pymodule]
fn toboggan_py(module: &Bound<'_, PyModule>) -> PyResult<()> {
    // The bindings and `toboggan-client` beneath them report over `tracing`,
    // which reaches `log`, which this hands to Python's own `logging`. An
    // importable extension module has no business writing to stdout: a caller
    // piping their script into `jq` should not have to filter our chatter out
    // of their data, and a caller who wants the chatter should be able to ask
    // for it with `logging.basicConfig(level=...)` like anything else.
    //
    // Ignored on a second import: re-initialising is not a reason to fail to
    // load the module.
    let _already_initialised = pyo3_log::try_init();

    module.add_class::<Talk>()?;
    module.add_class::<Slides>()?;
    module.add_class::<Slide>()?;
    // Registered but deliberately absent from `__all__`: it is what `iter()`
    // hands back, never something a caller names or constructs.
    module.add_class::<SlidesIter>()?;
    module.add_class::<State>()?;
    module.add_class::<ClientInfo>()?;
    module.add_class::<Toboggan>()?;

    // Maturin's generated package wrapper does `from .toboggan_py import *`
    // and re-exports this list, so without it `from toboggan_py import *`
    // falls back to "every public name" and the `__all__` in the type stub
    // describes something the module does not have.
    module.add(
        "__all__",
        vec!["ClientInfo", "Slide", "Slides", "State", "Talk", "Toboggan"],
    )?;

    Ok(())
}
