use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Content, Talk, TerminalConfig};

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
    ///
    /// When `Some`, `body` is expected to be the rendered projection of this
    /// source — `Slide::from_markdown` is the constructor that pairs them
    /// consistently; ad-hoc construction must uphold that invariant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_source: Option<String>,
    /// Targets this slide should be excluded from. Empty means visible everywhere.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub hidden_in: BTreeSet<RenderTarget>,
    /// Working directory for the `QuakeTerminal` overlay when this slide is active.
    /// If unset, falls back to [`Talk::default_terminal_cwd`], then to the server cwd.
    /// Resolved against [`Talk::source_dir`] when relative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quake_terminal_cwd: Option<String>,
    /// Lint rule ids silenced for this slide (front matter `disabled_rules` or a
    /// `<!-- lint-disable rule-id -->` body comment). Disabling is per-slide:
    /// diagnostics are not line-tracked, so a directive covers the whole slide.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lint_disabled: Vec<String>,
}

/// Borrowed view over a slide's body content.
///
/// Returned by [`Slide::body_view`]. Makes the three meaningful body states
/// explicit so callers do not have to inspect both `body` and `body_source`
/// fields and reason about whether they agree.
#[derive(Debug)]
pub enum SlideBody<'a> {
    /// No body content.
    Empty,
    /// Pre-rendered content, no markdown source available.
    Rendered(&'a Content),
    /// Body produced from markdown — both the source and its rendered form.
    FromMarkdown {
        source: &'a str,
        rendered: &'a Content,
    },
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

    /// Build a slide from a markdown source and its rendered projection.
    ///
    /// This is the blessed constructor for parser-produced slides: it pairs
    /// `body_source` and `body` so `body_view` returns the consistent
    /// `FromMarkdown` variant.
    #[must_use]
    pub fn from_markdown(source: impl Into<String>, rendered: impl Into<Content>) -> Self {
        Self {
            body: rendered.into(),
            body_source: Some(source.into()),
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

    /// Returns a view classifying the body as `Empty` / `Rendered` / `FromMarkdown`.
    ///
    /// Prefer this over inspecting `body` and `body_source` directly — the
    /// view encodes the only three combinations that should ever appear.
    #[must_use]
    pub fn body_view(&self) -> SlideBody<'_> {
        match (self.body_source.as_deref(), &self.body) {
            (Some(source), rendered) => SlideBody::FromMarkdown { source, rendered },
            (None, Content::Empty) => SlideBody::Empty,
            (None, rendered) => SlideBody::Rendered(rendered),
        }
    }

    #[must_use]
    pub fn with_quake_terminal_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.quake_terminal_cwd = Some(cwd.into());
        self
    }

    #[must_use]
    pub fn with_lint_disabled(mut self, rules: impl IntoIterator<Item = String>) -> Self {
        self.lint_disabled = Vec::from_iter(rules);
        self
    }

    /// Resolve the working directory for the `QuakeTerminal` overlay on this slide.
    ///
    /// Picks the first defined value among: this slide's `quake_terminal_cwd`,
    /// then `talk.default_terminal_cwd`. Relative paths are joined with
    /// `talk.source_dir` when available; absolute paths are returned unchanged.
    /// Returns `None` if no cwd is configured at any level (the server then uses its own cwd).
    #[must_use]
    pub fn resolved_quake_cwd(&self, talk: &Talk) -> Option<String> {
        let raw = self
            .quake_terminal_cwd
            .as_deref()
            .or(talk.default_terminal_cwd.as_deref())?;

        let path = PathBuf::from(raw);
        if path.is_absolute() {
            return Some(raw.to_owned());
        }
        match talk.source_dir.as_deref() {
            Some(dir) => Some(Path::new(dir).join(&path).to_string_lossy().into_owned()),
            None => Some(raw.to_owned()),
        }
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

    #[test]
    fn test_body_view_empty() {
        let slide = Slide::default();
        assert!(matches!(slide.body_view(), SlideBody::Empty));
    }

    #[test]
    fn test_body_view_rendered() {
        let slide = Slide::new(Content::text("Title")).with_body(Content::text("Hello"));
        match slide.body_view() {
            SlideBody::Rendered(Content::Text { text }) => assert_eq!(text, "Hello"),
            other => panic!("expected Rendered, got {other:?}"),
        }
    }

    #[test]
    fn test_body_view_from_markdown() {
        let slide = Slide::from_markdown("# Hi\n\nbody", Content::text("rendered"));
        match slide.body_view() {
            SlideBody::FromMarkdown { source, rendered } => {
                assert_eq!(source, "# Hi\n\nbody");
                assert!(matches!(rendered, Content::Text { text } if text == "rendered"));
            }
            other => panic!("expected FromMarkdown, got {other:?}"),
        }
    }

    #[test]
    fn test_from_markdown_pairs_body_and_source() {
        // The smart constructor must set both fields so direct field access
        // and body_view agree.
        let slide = Slide::from_markdown("source text", Content::text("rendered text"));
        assert_eq!(slide.body_source.as_deref(), Some("source text"));
        assert!(matches!(slide.body, Content::Text { ref text } if text == "rendered text"));
    }

    #[test]
    fn resolved_quake_cwd_returns_none_when_unset() {
        let talk = Talk::new("t");
        let slide = Slide::new("s");
        assert_eq!(slide.resolved_quake_cwd(&talk), None);
    }

    #[test]
    fn resolved_quake_cwd_uses_talk_default_when_slide_unset() {
        let talk = Talk::new("t").with_default_terminal_cwd("/tmp/foo");
        let slide = Slide::new("s");
        assert_eq!(slide.resolved_quake_cwd(&talk), Some("/tmp/foo".to_owned()));
    }

    #[test]
    fn resolved_quake_cwd_slide_overrides_talk_default() {
        let talk = Talk::new("t").with_default_terminal_cwd("/tmp/default");
        let slide = Slide::new("s").with_quake_terminal_cwd("/tmp/slide");
        assert_eq!(
            slide.resolved_quake_cwd(&talk),
            Some("/tmp/slide".to_owned())
        );
    }

    #[test]
    fn resolved_quake_cwd_joins_relative_with_source_dir() {
        let talk = Talk::new("t").with_source_dir("/talks/demo");
        let slide = Slide::new("s").with_quake_terminal_cwd("examples/api");
        let resolved = slide.resolved_quake_cwd(&talk).expect("some");
        // Use Path equality to keep the test cross-platform.
        assert_eq!(
            PathBuf::from(resolved),
            PathBuf::from("/talks/demo/examples/api")
        );
    }

    #[test]
    fn resolved_quake_cwd_keeps_absolute_path_unchanged_with_source_dir() {
        let talk = Talk::new("t").with_source_dir("/talks/demo");
        let slide = Slide::new("s").with_quake_terminal_cwd("/etc");
        assert_eq!(slide.resolved_quake_cwd(&talk), Some("/etc".to_owned()));
    }

    #[test]
    fn resolved_quake_cwd_returns_relative_unchanged_without_source_dir() {
        let talk = Talk::new("t");
        let slide = Slide::new("s").with_quake_terminal_cwd("examples/api");
        assert_eq!(
            slide.resolved_quake_cwd(&talk),
            Some("examples/api".to_owned())
        );
    }
}
