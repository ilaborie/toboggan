use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::Style;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type")]
pub enum Content {
    #[default]
    Empty,
    Text {
        text: String,
    },
    Html {
        raw: String,
        #[serde(default, skip_serializing_if = "Style::is_default")]
        style: Style,
        #[serde(skip_serializing_if = "Option::is_none")]
        alt: Option<String>,
    },
}

impl Content {
    pub(crate) fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn text(text: impl Into<String>) -> Self {
        let text = text.into();
        Self::Text { text }
    }

    pub fn html(raw: impl Into<String>) -> Self {
        let style = Style::default();
        let raw = raw.into();
        let alt = None;
        Self::Html { raw, alt, style }
    }

    pub fn html_with_alt(raw: impl Into<String>, alt: impl Into<String>) -> Self {
        let style = Style::default();
        let raw = raw.into();
        let alt = Some(alt.into());
        Self::Html { raw, alt, style }
    }

    /// The text to show a human: an HTML fragment's alt text when it has one,
    /// its markup otherwise, and nothing at all when there is no content.
    ///
    /// This three-arm match was written out in nine places across the workspace
    /// and the copies disagreed — the clients preferred `alt`, the HTML and
    /// thumbnail renderers ignored it — so the same slide described itself
    /// differently depending on who was asking.
    #[must_use]
    pub fn display_text(&self) -> &str {
        match self {
            Self::Empty => "",
            Self::Text { text } => text,
            Self::Html { raw, alt, .. } => alt.as_deref().unwrap_or(raw),
        }
    }

    /// The markup as rendered, ignoring any alt text.
    ///
    /// For anything that inspects or emits HTML, where the alt text is a
    /// description of the content rather than the content itself.
    #[must_use]
    pub fn raw_html(&self) -> &str {
        match self {
            Self::Empty => "",
            Self::Text { text } => text,
            Self::Html { raw, .. } => raw,
        }
    }
}

impl From<&str> for Content {
    fn from(text: &str) -> Self {
        Self::text(text)
    }
}

impl From<String> for Content {
    fn from(text: String) -> Self {
        Self::text(text)
    }
}

impl Display for Content {
    /// Empty content formats as nothing.
    ///
    /// It used to format as the literal `<no content>`, which most callers did
    /// not guard against: an untitled part reached the stats table, the TUI's
    /// slide list and the `titles` array of `GET /api/talk` with that string as
    /// its name. A placeholder belongs where a caller decides one is wanted,
    /// not in the formatting of the value.
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "{}", self.display_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_text_prefers_alt_over_markup() {
        let content = Content::html_with_alt("<p>Hi</p>", "Hi");
        assert_eq!(content.display_text(), "Hi");
        assert_eq!(content.raw_html(), "<p>Hi</p>");
    }

    #[test]
    fn display_text_falls_back_to_markup_without_alt() {
        let content = Content::html("<p>Hi</p>");
        assert_eq!(content.display_text(), "<p>Hi</p>");
        assert_eq!(content.raw_html(), "<p>Hi</p>");
    }

    /// The placeholder this used to produce reached the stats table, the TUI's
    /// slide list and the `titles` array of `GET /api/talk` as a slide's name.
    #[test]
    fn empty_content_has_no_placeholder_text() {
        assert_eq!(Content::Empty.display_text(), "");
        assert_eq!(Content::Empty.raw_html(), "");
    }
}
