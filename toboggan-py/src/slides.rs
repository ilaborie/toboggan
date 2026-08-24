use pyo3::prelude::*;
use toboggan_core::{SlideKind, SlidesResponse};

/// The wire spelling of a slide kind, for the same reason as `role_name`.
///
/// `Debug` is not an API. Deriving the Python contract from it made renaming a
/// `SlideKind` variant a silent breaking change for every caller, with nothing
/// in the workspace to catch it; a total match makes it a compile error here.
/// Lower-case to match `hidden_in`, the front matter and serde's own spelling.
pub(crate) const fn kind_name(kind: SlideKind) -> &'static str {
    match kind {
        SlideKind::Cover => "cover",
        SlideKind::Part => "part",
        SlideKind::Standard => "standard",
    }
}

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

    fn __getitem__(&self, index: isize) -> PyResult<Slide> {
        self.resolve(index)
            .and_then(|index| self.0.slides.get(index).cloned())
            .map(Slide)
            .ok_or_else(|| pyo3::exceptions::PyIndexError::new_err("slide index out of range"))
    }

    /// Get a slide by index, returns None if out of range.
    fn get(&self, index: isize) -> Option<Slide> {
        self.resolve(index)
            .and_then(|index| self.0.slides.get(index).cloned())
            .map(Slide)
    }

    /// Iterates the slides in order.
    ///
    /// A real iterator rather than a reliance on the legacy `__getitem__`
    /// protocol. That protocol does work at runtime — pyo3 fills in `sq_item`
    /// — but it is invisible to a type-checker, so `for slide in client.slides`
    /// was an error against the stub while running perfectly well.
    fn __iter__(&self) -> SlidesIter {
        SlidesIter {
            slides: self.0.slides.clone(),
            next: 0,
        }
    }
}

impl Slides {
    /// A Python index as a position in the vector.
    ///
    /// Negative indices count from the end, as they do for every other sequence
    /// in the language. Taking `usize` instead made `slides[-1]` an
    /// `OverflowError` from the argument conversion — not the `IndexError` the
    /// stub promised, and not what anyone writing Python expects.
    fn resolve(&self, index: isize) -> Option<usize> {
        let len = self.0.slides.len();
        if index >= 0 {
            return usize::try_from(index).ok();
        }
        // `-len` is the first slide and anything before it is out of range;
        // computed in `isize` so a huge negative cannot wrap.
        isize::try_from(len)
            .ok()
            .and_then(|len| len.checked_add(index))
            .and_then(|from_end| usize::try_from(from_end).ok())
    }
}

/// Iterator over [`Slides`], holding its own copy.
///
/// The deck is tens of slides, and a snapshot means iteration cannot be
/// disturbed by a reload landing mid-loop.
#[pyclass]
pub struct SlidesIter {
    slides: Vec<toboggan_core::Slide>,
    next: usize,
}

#[pymethods]
impl SlidesIter {
    const fn __iter__(this: PyRef<'_, Self>) -> PyRef<'_, Self> {
        this
    }

    fn __next__(&mut self) -> Option<Slide> {
        let slide = self.slides.get(self.next).cloned().map(Slide)?;
        self.next += 1;
        Some(slide)
    }
}

/// A single slide in the presentation.
#[pyclass]
pub struct Slide(pub(crate) toboggan_core::Slide);

#[pymethods]
impl Slide {
    /// Whether this is the `"cover"`, a `"part"` title, or a `"standard"` slide.
    #[getter]
    const fn kind(&self) -> &'static str {
        kind_name(self.0.kind)
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

    /// Render targets this slide is excluded from — in practice only `"pdf"`.
    ///
    /// The server serves `visible_in(RenderTarget::Web)`, so web-hidden slides
    /// never reach a client at all and `"web"` cannot appear here. Empty means
    /// the slide is in the PDF too.
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
