use pyo3::prelude::*;
use toboggan_core::SlidesResponse;

/// Collection of slides in the presentation.
#[pyclass]
pub struct Slides(pub(crate) SlidesResponse);

#[pymethods]
impl Slides {
    fn __repr__(&self) -> String {
        let count = self.0.slides.len();
        let slides = self
            .0
            .slides
            .iter()
            .enumerate()
            .map(|(i, slide)| format!("  {}: {slide}", i + 1))
            .collect::<Vec<_>>()
            .join("\n");
        format!("Slides({count}):\n{slides}")
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __len__(&self) -> usize {
        self.0.slides.len()
    }

    fn __getitem__(&self, index: usize) -> PyResult<Slide> {
        self.0
            .slides
            .get(index)
            .cloned()
            .map(Slide)
            .ok_or_else(|| pyo3::exceptions::PyIndexError::new_err("slide index out of range"))
    }

    /// Get a slide by index, returns None if out of range.
    fn get(&self, index: usize) -> Option<Slide> {
        self.0.slides.get(index).cloned().map(Slide)
    }
}

/// A single slide in the presentation.
#[pyclass]
pub struct Slide(pub(crate) toboggan_core::Slide);

#[pymethods]
impl Slide {
    #[getter]
    fn kind(&self) -> String {
        format!("{:?}", self.0.kind)
    }

    #[getter]
    fn title(&self) -> String {
        self.0.title.to_string()
    }

    #[getter]
    fn body(&self) -> String {
        self.0.body.to_string()
    }

    #[getter]
    fn notes(&self) -> String {
        self.0.notes.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Slide({:?}, \"{}\")", self.0.kind, self.0)
    }
}
