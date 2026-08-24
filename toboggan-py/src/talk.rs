use std::time::Duration;

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
            .map_or(String::new(), |footer| format!("\n  footer: {footer}"));
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

    /// The deck's BCP 47 language tag, or None when it did not declare one.
    #[getter]
    fn lang(&self) -> Option<&str> {
        self.0.lang.as_deref()
    }

    /// The slide titles.
    #[getter]
    fn titles(&self) -> Vec<String> {
        self.0.titles.clone()
    }

    /// Step counts per slide for animation progress.
    ///
    /// Always as long as `titles`; see [`Talk::durations`].
    #[getter]
    fn step_counts(&self) -> Vec<usize> {
        pad_to(self.0.step_counts.clone(), self.0.titles.len(), 0)
    }

    /// Planned speaking time per slide, in seconds, from each slide's
    /// `duration` front matter. None where the author did not say.
    ///
    /// Always as long as `titles` and read against it by index — which is the
    /// whole plan a presenter view needs to work out whether the talk is
    /// running early or late.
    ///
    /// The wire form is *either* empty *or* one per slide, and the empty case
    /// means "not computed" rather than "no durations". Left as-is, the obvious
    /// `zip(talk.titles, talk.durations)` silently yields nothing at all in that
    /// case, so it is padded here: one representation, and the length relation
    /// the docs promise holds unconditionally.
    #[getter]
    fn durations(&self) -> Vec<Option<f64>> {
        let seconds = self
            .0
            .durations
            .iter()
            // Through `Duration`, the same route `Slide.duration` takes, so the
            // two agree by construction rather than by coincidence.
            .map(|duration| duration.map(|secs| Duration::from_secs(secs).as_secs_f64()))
            .collect();
        // Seconds as a float, matching `Slide.duration` — the same quantity
        // reported as an int here and a float there invited comparisons that
        // only happened to work.
        pad_to(seconds, self.0.titles.len(), None)
    }

    /// Optional HTML head content.
    #[getter]
    fn head(&self) -> Option<&str> {
        self.0.head.as_deref()
    }
}

/// A per-slide vector at exactly one entry per slide.
///
/// The wire form of these is either empty or already the right length, so this
/// only ever fills in the "not computed" case. Truncation is defensive: a
/// longer-than-expected vector would break the same indexing promise.
fn pad_to<T: Clone>(mut values: Vec<T>, slides: usize, missing: T) -> Vec<T> {
    values.resize(slides, missing);
    values
}
