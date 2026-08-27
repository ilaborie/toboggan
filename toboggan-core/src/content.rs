use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::Style;

/// Anything a slide can say: its title, its body, or its speaker notes.
///
/// Use [`Self::display_text`] to ask what a piece of content *says*, rather
/// than matching on the variants — the variants exist so that a renderer which
/// can show markup gets markup, and everything else gets words.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type")]
pub enum Content {
    /// Nothing at all — an absent title, a slide with no notes.
    #[default]
    Empty,
    /// Plain text, with no markup to interpret.
    Text {
        /// The text itself.
        text: String,
    },
    /// An HTML fragment, as produced by rendering a slide's Markdown.
    Html {
        /// The markup, ready to be placed into the document.
        raw: String,
        /// Classes and inline style to apply where this is rendered.
        #[serde(default, skip_serializing_if = "Style::is_default")]
        style: Style,
        /// A plain-text description, for anything that cannot show markup —
        /// the terminal client, the stats table, a screen reader.
        #[serde(skip_serializing_if = "Option::is_none")]
        alt: Option<String>,
    },
}

impl Content {
    /// Whether this is the [`Self::Empty`] variant.
    ///
    /// The structural question, asked by serde: `Slide` skips serializing an
    /// absent title or absent notes with it. Callers deciding whether to *draw*
    /// something want [`Self::is_blank`] instead — this one is `false` for a
    /// `Text` holding `""`, which says nothing while being something.
    pub(crate) fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Whether this content would show a reader nothing at all.
    ///
    /// Broader than the crate-private `is_empty`, and the question every client is
    /// actually asking when it decides whether to draw a heading, a notes box or
    /// a list entry: an `Html` fragment whose alt text is empty and a `Text`
    /// holding `""` both say nothing.
    ///
    /// It exists because the desktop client asked it as
    /// `matches!(.., Content::Text { text } if text.is_empty())`, in five
    /// places — which is `false` for `Content::Empty`, the variant the server
    /// actually sends for an absent title or absent notes, precisely because
    /// `is_empty` told it to skip them. Every slide without notes was
    /// therefore drawn with an empty notes box, and every untitled slide with a
    /// blank heading.
    #[must_use]
    pub fn is_blank(&self) -> bool {
        self.display_text().is_empty()
    }

    /// Plain text content.
    pub fn text(text: impl Into<String>) -> Self {
        let text = text.into();
        Self::Text { text }
    }

    /// An HTML fragment with no alt text, so [`Self::display_text`] falls back
    /// to the markup.
    pub fn html(raw: impl Into<String>) -> Self {
        let style = Style::default();
        let raw = raw.into();
        let alt = None;
        Self::Html { raw, alt, style }
    }

    /// An HTML fragment together with the plain text that stands in for it.
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

    /// The desktop client asked this as `matches!(.., Content::Text { text } if
    /// text.is_empty())`, which is `false` for the one variant the server
    /// actually sends for absent content.
    #[test]
    fn every_variant_that_says_nothing_is_blank() {
        assert!(Content::Empty.is_blank());
        assert!(Content::text("").is_blank());
        assert!(Content::html("").is_blank());
        assert!(Content::html_with_alt("<p></p>", "").is_blank());
    }

    #[test]
    fn content_with_words_is_not_blank() {
        assert!(!Content::text("Hi").is_blank());
        assert!(!Content::html("<p>Hi</p>").is_blank());
        assert!(!Content::html_with_alt("<p>Hi</p>", "Hi").is_blank());
    }

    /// The two questions differ on exactly one case, and that case is why
    /// `is_blank` exists rather than `is_empty` being made public.
    #[test]
    fn empty_text_is_blank_without_being_empty() {
        let content = Content::text("");
        assert!(content.is_blank());
        assert!(!content.is_empty());
    }
}
