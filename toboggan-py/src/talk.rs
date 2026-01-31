use pyo3::prelude::*;
use toboggan_core::TalkResponse;

/// Presentation metadata.
#[pyclass]
pub struct Talk(pub(crate) TalkResponse);

#[pymethods]
impl Talk {
    fn __repr__(&self) -> String {
        let title = &self.0.title;
        let date = &self.0.date;
        let slide_count = self.0.titles.len();
        let footer = self
            .0
            .footer
            .as_ref()
            .map_or(String::new(), |f| format!("\n  footer: {f}"));
        format!("Talk(\"{title}\", {date}, {slide_count} slides){footer}")
    }

    fn __str__(&self) -> String {
        self.0.title.clone()
    }

    /// The presentation title.
    #[getter]
    fn title(&self) -> &str {
        &self.0.title
    }

    /// The presentation date.
    #[getter]
    fn date(&self) -> String {
        self.0.date.to_string()
    }

    /// The optional footer text.
    #[getter]
    fn footer(&self) -> Option<&str> {
        self.0.footer.as_deref()
    }

    /// The slide titles.
    #[getter]
    fn titles(&self) -> Vec<String> {
        self.0.titles.clone()
    }

    /// Step counts per slide for animation progress.
    #[getter]
    fn step_counts(&self) -> Vec<usize> {
        self.0.step_counts.clone()
    }

    /// Optional HTML head content.
    #[getter]
    fn head(&self) -> Option<&str> {
        self.0.head.as_deref()
    }
}
