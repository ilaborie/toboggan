use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::{Content, TerminalConfig};

/// A type-safe identifier for slides in a presentation.
///
/// `SlideId` wraps a `usize` to provide type safety when indexing into slide collections.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(transparent)]
pub struct SlideId(usize);

impl SlideId {
    /// The first slide (index 0)
    pub const FIRST: Self = Self(0);

    /// Creates a new `SlideId` from a raw index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the raw index value.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }

    /// Returns the previous slide ID, or `None` if this is the first slide.
    #[must_use]
    pub fn prev(self) -> Option<Self> {
        self.0.checked_sub(1).map(Self)
    }

    /// Returns the 1-based slide number for display purposes.
    #[must_use]
    pub const fn display_number(self) -> usize {
        self.0 + 1
    }
}

impl Display for SlideId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Display as 1-indexed for human readability
        write!(f, "{}", self.0 + 1)
    }
}

impl From<usize> for SlideId {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

impl From<SlideId> for usize {
    fn from(id: SlideId) -> Self {
        id.0
    }
}

/// Rendering targets for per-slide visibility control.
///
/// Used with `hidden_in` on a slide to exclude it from specific outputs.
/// An empty set means the slide is visible in all targets.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum RenderTarget {
    /// Web / HTML output
    Web,
    /// PDF output (Typst)
    Pdf,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(default)]
pub struct Slide {
    pub kind: SlideKind,
    #[serde(skip_serializing_if = "Style::is_default")]
    pub style: Style,
    #[serde(skip_serializing_if = "Content::is_empty")]
    pub title: Content,
    pub body: Content,
    #[serde(skip_serializing_if = "Content::is_empty")]
    pub notes: Content,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terminals: Vec<TerminalConfig>,
    /// Raw markdown source of the slide body, used by non-HTML exporters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_source: Option<String>,
    /// Targets this slide should be excluded from. Empty means visible everywhere.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub hidden_in: BTreeSet<RenderTarget>,
}

impl Slide {
    pub fn new(title: impl Into<Content>) -> Self {
        let title = title.into();
        Self {
            title,
            ..Default::default()
        }
    }

    pub fn cover(title: impl Into<Content>) -> Self {
        let title = title.into();
        Self {
            kind: SlideKind::Cover,
            title,
            ..Default::default()
        }
    }

    pub fn part(title: impl Into<Content>) -> Self {
        let title = title.into();
        Self {
            kind: SlideKind::Part,
            title,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_style_classes(mut self, classes: impl IntoIterator<Item = String>) -> Self {
        self.style.classes = Vec::from_iter(classes);
        self
    }

    #[must_use]
    pub fn with_body(mut self, body: impl Into<Content>) -> Self {
        self.body = body.into();
        self
    }

    #[must_use]
    pub fn with_notes(mut self, notes: impl Into<Content>) -> Self {
        self.notes = notes.into();
        self
    }

    #[must_use]
    pub fn with_terminal(mut self, terminal: TerminalConfig) -> Self {
        self.terminals.push(terminal);
        self
    }

    #[must_use]
    pub fn with_hidden_in(mut self, targets: impl IntoIterator<Item = RenderTarget>) -> Self {
        self.hidden_in = targets.into_iter().collect();
        self
    }

    /// Returns `true` if the slide should be excluded from the given render target.
    #[must_use]
    pub fn is_hidden_from(&self, target: RenderTarget) -> bool {
        self.hidden_in.contains(&target)
    }
}

impl Display for Slide {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        if let Content::Empty = self.title {
            write!(fmt, "{}", self.body)
        } else {
            write!(fmt, "{}", self.title)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum SlideKind {
    Cover,
    Part,
    #[default]
    Standard,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Style {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

impl Style {
    pub(crate) fn is_default(&self) -> bool {
        self.classes.is_empty() && self.style.is_none()
    }
}

impl Display for Style {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let classes = self.classes.join(" ");
        write!(fmt, "{classes}")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SlidesResponse {
    pub slides: Vec<Slide>,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_slide_id_new_and_index() {
        let id = SlideId::new(5);
        assert_eq!(id.index(), 5);
    }

    #[test]
    fn test_slide_id_first() {
        assert_eq!(SlideId::FIRST.index(), 0);
    }

    #[test]
    fn test_slide_id_display() {
        // Display should be 1-indexed for human readability
        let id = SlideId::new(0);
        assert_eq!(format!("{id}"), "1");

        let id = SlideId::new(9);
        assert_eq!(format!("{id}"), "10");
    }

    #[test]
    fn test_slide_id_from_usize() {
        let id: SlideId = 42.into();
        assert_eq!(id.index(), 42);
    }

    #[test]
    fn test_usize_from_slide_id() {
        let id = SlideId::new(42);
        let index: usize = id.into();
        assert_eq!(index, 42);
    }

    #[test]
    fn test_slide_id_serde() {
        let id = SlideId::new(42);
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "42");

        let parsed: SlideId = serde_json::from_str("42").expect("deserialize");
        assert_eq!(parsed, id);
    }

    #[test]
    fn test_slide_id_default() {
        let id = SlideId::default();
        assert_eq!(id.index(), 0);
    }

    #[test]
    fn test_slide_id_ordering() {
        let id1 = SlideId::new(1);
        let id2 = SlideId::new(2);
        let id3 = SlideId::new(1);

        assert!(id1 < id2);
        assert!(id2 > id1);
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_slide_id_prev() {
        assert_eq!(SlideId::FIRST.prev(), None);
        assert_eq!(SlideId::new(1).prev(), Some(SlideId::FIRST));
        assert_eq!(SlideId::new(5).prev(), Some(SlideId::new(4)));
    }

    #[test]
    fn test_slide_id_display_number() {
        assert_eq!(SlideId::FIRST.display_number(), 1);
        assert_eq!(SlideId::new(0).display_number(), 1);
        assert_eq!(SlideId::new(9).display_number(), 10);
    }
}
