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
    /// Whether this is the `"Cover"`, a `"Part"` title, or a `"Standard"` slide.
    #[getter]
    fn kind(&self) -> String {
        format!("{:?}", self.0.kind)
    }

    /// The slide's heading, as words rather than markup.
    #[getter]
    fn title(&self) -> &str {
        self.0.title.display_text()
    }

    /// Everything below the heading.
    #[getter]
    fn body(&self) -> &str {
        self.0.body.display_text()
    }

    /// Speaker notes — never shown on the projector.
    #[getter]
    fn notes(&self) -> &str {
        self.0.notes.display_text()
    }

    /// Speaking time the author planned for this slide, in seconds, or None
    /// where the front matter did not declare one.
    #[getter]
    fn duration(&self) -> Option<f64> {
        self.0.duration.map(|duration| duration.as_secs_f64())
    }

    /// Render targets this slide is excluded from — `"web"`, `"pdf"`. Empty
    /// means it is visible everywhere.
    #[getter]
    fn hidden_in(&self) -> Vec<String> {
        // `RenderTarget` is `#[non_exhaustive]`, so a match would need a
        // wildcard arm that names a target it cannot know. Lower-casing the
        // variant is the spelling serde already uses on the wire and the one
        // the front matter is written in, and it stays right when a target is
        // added.
        self.0
            .hidden_in
            .iter()
            .map(|target| format!("{target:?}").to_lowercase())
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("Slide({:?}, \"{}\")", self.0.kind, self.0)
    }
}
