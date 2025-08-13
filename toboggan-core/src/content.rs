//! Content types and rendering support for slides.
//!
//! This module provides the [`Content`] enum and related types for representing
//! rich content within slides. Content can be text, HTML, embedded iframes,
//! terminal sessions (std only), or layout containers.

#[cfg(feature = "openapi")]
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Display;
#[cfg(feature = "std")]
use std::path::{Path, PathBuf};
use std::string::ToString;

use serde::{Deserialize, Serialize};

use crate::Style;

/// Working directory path for terminal content.
///
/// A simple wrapper around `PathBuf` for std environments and `String` for `no_std` environments.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkingDirectory(String);

impl WorkingDirectory {
    /// Creates a new working directory from a path-like input.
    pub fn new(path: impl Into<Self>) -> Self {
        path.into()
    }

    /// Returns the path as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for WorkingDirectory {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "{}", self.as_str())
    }
}

impl From<&str> for WorkingDirectory {
    fn from(path: &str) -> Self {
        Self(path.to_string())
    }
}

/// Rich content that can be displayed in a slide.
///
/// Content supports various media types and layout containers. All content types
/// can be serialized to JSON and displayed across different renderers.
///
/// # Examples
///
/// ```rust
/// use toboggan_core::Content;
///
/// // Simple text
/// let text = Content::from("Hello, world!");
///
/// // HTML content
/// let html = Content::html("<img src='chart.png' alt='Sales chart showing upward trend'>");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type")]
pub enum Content {
    /// Empty content (default).
    #[default]
    Empty,

    /// Plain text content.
    ///
    /// The text will be rendered as-is, with appropriate escaping
    /// applied by the renderer to prevent XSS attacks.
    Text { text: String },

    /// HTML content with optional alt text for accessibility.
    ///
    /// The `raw` field contains the HTML markup, while the optional `alt`
    /// field provides a text alternative for screen readers and non-HTML
    /// renderers.
    ///
    /// **Security Note**: HTML content should be sanitized before display
    /// to prevent XSS attacks. The WASM client includes comprehensive
    /// HTML sanitization.
    Html { raw: String, alt: Option<String> },

    /// Grid layout container.
    Grid {
        style: Style,
        contents: Vec<Content>,
    },

    /// Embedded iframe content.
    ///
    /// Displays content from an external URL within an iframe.
    /// Useful for embedding videos, interactive content, or external sites.
    IFrame { url: String },

    /// Terminal session.
    ///
    /// Provides an interactive terminal within the slide, starting
    /// in the specified working directory. Useful for live coding
    /// demonstrations and command-line tutorials.
    ///
    /// Available in both `std` and `no_std` environments:
    /// - `std`: Uses proper path handling with `PathBuf`
    /// - `no_std`: Uses string-based path storage
    Term { cwd: WorkingDirectory },
}

impl Content {
    /// Creates plain text content.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Content;
    ///
    /// let content = Content::text("Hello, world!");
    /// let content2 = Content::text(String::from("Dynamic text"));
    /// ```
    pub fn text(text: impl Into<String>) -> Self {
        let text = text.into();
        Self::Text { text }
    }

    /// Creates HTML content without alt text.
    ///
    /// For accessibility, consider using [`Content::html_with_alt`] when
    /// the HTML contains important visual information.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Content;
    ///
    /// let content = Content::html("<h1>Title</h1>");
    /// let content2 = Content::html(format!("<p>Count: {}</p>", 42));
    /// ```
    pub fn html(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let alt = None;
        Self::Html { raw, alt }
    }

    /// Creates HTML content with alt text for accessibility.
    ///
    /// The alt text provides a description of the HTML content for
    /// screen readers and text-only renderers.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Content;
    ///
    /// let content = Content::html_with_alt(
    ///     "<img src='chart.png' width='400'>",
    ///     "Bar chart showing sales growth from Q1 to Q4"
    /// );
    /// ```
    pub fn html_with_alt(raw: impl Into<String>, alt: impl Into<String>) -> Self {
        let raw = raw.into();
        let alt = Some(alt.into());
        Self::Html { raw, alt }
    }

    /// Creates grid layout content.
    ///
    /// Provides a CSS Grid layout using the provided style and contents.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::{Content, Style};
    ///
    /// let style = Style::default();
    /// let grid = Content::grid(style, [
    ///     Content::from("Cell 1"),
    ///     Content::from("Cell 2")
    /// ]);
    /// ```
    pub fn grid(style: Style, contents: impl IntoIterator<Item = Content>) -> Self {
        let contents = Vec::from_iter(contents);
        Self::Grid { style, contents }
    }

    /// Creates horizontal layout content.
    ///
    /// This is a convenience method that creates a grid with horizontal layout.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Content;
    ///
    /// let content = Content::hbox([
    ///     Content::from("Left column"),
    ///     Content::from("Right column")
    /// ]);
    /// ```
    pub fn hbox(contents: impl IntoIterator<Item = Content>) -> Self {
        Self::grid(Style::default(), contents)
    }

    /// Creates vertical layout content.
    ///
    /// This is a convenience method that creates a grid with vertical layout.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Content;
    ///
    /// let content = Content::vbox([
    ///     Content::html("<header>Header</header>"),
    ///     Content::from("Main content area"),
    ///     Content::html("<footer>Footer</footer>")
    /// ]);
    /// ```
    pub fn vbox(contents: impl IntoIterator<Item = Content>) -> Self {
        Self::grid(Style::default(), contents)
    }

    /// Creates iframe content for embedding external URLs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Content;
    ///
    /// let content = Content::iframe("https://example.com");
    /// let video = Content::iframe("https://youtube.com/embed/dQw4w9WgXcQ");
    /// ```
    pub fn iframe(url: impl Into<String>) -> Self {
        let url = url.into();
        Self::IFrame { url }
    }

    /// Creates terminal content.
    ///
    /// The terminal will start in the specified working directory.
    /// Works in both `std` and `no_std` environments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Content;
    ///
    /// let content = Content::term("/home/user/project");
    /// let content2 = Content::term(String::from("./demo"));
    /// ```
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "std")]
    /// # {
    /// use toboggan_core::Content;
    /// use std::path::Path;
    ///
    /// let content3 = Content::term(Path::new("./demo"));
    /// # }
    /// ```
    pub fn term(cwd: impl Into<WorkingDirectory>) -> Self {
        let cwd = cwd.into();
        Self::Term { cwd }
    }
}

/// Converts a string slice into text content.
///
/// This is a convenience implementation that creates [`Content::Text`].
///
/// # Examples
///
/// ```rust
/// use toboggan_core::Content;
///
/// let content: Content = "Hello, world!".into();
/// // Equivalent to: Content::text("Hello, world!")
/// ```
impl From<&str> for Content {
    fn from(text: &str) -> Self {
        Self::text(text)
    }
}

/// Converts an owned string into text content.
///
/// This is a convenience implementation that creates [`Content::Text`].
///
/// # Examples
///
/// ```rust
/// use toboggan_core::Content;
///
/// let content: Content = String::from("Hello, world!").into();
/// // Equivalent to: Content::text("Hello, world!")
/// ```
impl From<String> for Content {
    fn from(text: String) -> Self {
        Self::text(text)
    }
}

impl Display for Content {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => write!(fmt, "<no content>"),
            Self::Text { text } => write!(fmt, "{text}"),
            Self::Html { raw, alt } => {
                if let Some(alt) = alt {
                    write!(fmt, "{alt}")
                } else {
                    write!(fmt, "{raw}")
                }
            }
            Self::IFrame { url } => write!(fmt, "{url}"),
            Self::Grid { contents, .. } => {
                let mut first = true;
                for content in contents {
                    if first {
                        first = false;
                    } else {
                        write!(fmt, " - ")?;
                    }

                    write!(fmt, "{content}")?;
                }
                Ok(())
            }
            Self::Term { cwd } => {
                write!(fmt, "{cwd}")
            }
        }
    }
}

/// Converts a file path into appropriate content (only available with `std` feature).
///
/// This implementation automatically detects the file type based on the extension
/// and creates the appropriate content:
///
/// - `.html`, `.htm` → [`Content::Html`] with file content as raw HTML
/// - `.md`, `.markdown` → [`Content::Html`] with converted HTML and original Markdown as alt text
/// - Other extensions → [`Content::Text`] with file content as plain text
///
/// # Panics
///
/// Panics if the file cannot be read or if the Markdown parsing fails.
///
/// # Examples
///
/// ```rust,no_run
/// # #[cfg(feature = "std")]
/// # {
/// use toboggan_core::Content;
/// use std::path::Path;
///
/// // Automatically detects file type
/// let html_content: Content = Path::new("slides/intro.html").into();
/// let md_content: Content = Path::new("slides/overview.md").into();
/// let text_content: Content = Path::new("notes.txt").into();
/// # }
/// ```
#[cfg(feature = "std")]
impl From<&Path> for Content {
    fn from(path: &Path) -> Self {
        use pulldown_cmark::{Options, Parser, html};

        // Read the file content, panic if it fails
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("Failed to read file {}: {}", path.display(), err));

        // Get file extension to determine content type
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        match extension.to_lowercase().as_str() {
            "html" | "htm" => {
                // For HTML files, use content directly
                Self::Html {
                    raw: content,
                    alt: None,
                }
            }
            "md" | "markdown" => {
                // For Markdown files, convert to HTML and use original markdown as alt text
                let options = Options::all();
                let parser = Parser::new_ext(&content, options);

                let mut html_output = String::new();
                html::push_html(&mut html_output, parser);

                Self::Html {
                    raw: html_output,
                    alt: Some(content),
                }
            }
            _ => {
                // For other file types, treat as plain text
                Self::text(content)
            }
        }
    }
}

/// Converts an owned path into content (only available with `std` feature).
///
/// This is a convenience implementation that delegates to [`From<&Path>`].
///
/// # Examples
///
/// ```rust,no_run
/// # #[cfg(feature = "std")]
/// # {
/// use toboggan_core::Content;
/// use std::path::PathBuf;
///
/// let path = PathBuf::from("slides/intro.md");
/// let content: Content = path.into();
/// # }
/// ```
#[cfg(feature = "std")]
impl From<PathBuf> for Content {
    fn from(path: PathBuf) -> Self {
        Self::from(path.as_path())
    }
}

#[cfg(all(test, feature = "std"))]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let content = Content::from("test string");
        match content {
            Content::Text { text } => assert_eq!(text, "test string"),
            _ => panic!("Expected Text content"),
        }
    }
}
