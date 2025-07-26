//! Content types and rendering support for slides.
//!
//! This module provides the [`Content`] enum and related types for representing
//! rich content within slides. Content can be text, HTML, embedded iframes,
//! terminal sessions (std only), or layout containers.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Display;
#[cfg(feature = "std")]
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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
/// // HTML with accessibility
/// let html = Content::html_with_alt(
///     "<img src='chart.png'>",
///     "Sales chart showing upward trend"
/// );
///
/// // Layout containers
/// let layout = Content::hbox("1fr 2fr", [
///     Content::from("Left sidebar"),
///     Content::html("<main>Main content</main>")
/// ]);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

    /// Embedded iframe content.
    ///
    /// Displays content from an external URL within an iframe.
    /// Useful for embedding videos, interactive content, or external sites.
    IFrame { url: String },

    /// Terminal session (only available with `std` feature).
    ///
    /// Provides an interactive terminal within the slide, starting
    /// in the specified working directory. Useful for live coding
    /// demonstrations and command-line tutorials.
    #[cfg(feature = "std")]
    Term { cwd: PathBuf },

    /// Horizontal layout container.
    ///
    /// Arranges child content in a horizontal row. The `columns` field
    /// defines the sizing using CSS Grid syntax (e.g., "1fr 2fr" for
    /// a 1:2 ratio).
    HBox {
        columns: String,
        contents: Vec<Content>,
    },

    /// Vertical layout container.
    ///
    /// Arranges child content in a vertical column. The `rows` field
    /// defines the sizing using CSS Grid syntax (e.g., "auto 1fr auto"
    /// for header, content, footer).
    VBox {
        rows: String,
        contents: Vec<Content>,
    },
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

    /// Creates terminal content (only available with `std` feature).
    ///
    /// The terminal will start in the specified working directory.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "std")]
    /// # {
    /// use toboggan_core::Content;
    /// use std::path::Path;
    ///
    /// let content = Content::term("/home/user/project");
    /// let content2 = Content::term(Path::new("./demo"));
    /// # }
    /// ```
    #[cfg(feature = "std")]
    pub fn term(cwd: impl Into<PathBuf>) -> Self {
        let cwd = cwd.into();
        Self::Term { cwd }
    }

    /// Creates horizontal layout content.
    ///
    /// The `columns` parameter uses CSS Grid syntax to define column sizes.
    /// Common patterns:
    /// - `"1fr 1fr"` - Equal width columns
    /// - `"200px 1fr"` - Fixed width sidebar, flexible content
    /// - `"1fr 2fr 1fr"` - Three columns with 1:2:1 ratio
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Content;
    ///
    /// let content = Content::hbox("1fr 1fr", [
    ///     Content::from("Left column"),
    ///     Content::from("Right column")
    /// ]);
    ///
    /// let sidebar = Content::hbox("250px 1fr", [
    ///     Content::html("<nav>Navigation</nav>"),
    ///     Content::html("<main>Content</main>")
    /// ]);
    /// ```
    pub fn hbox(columns: impl Into<String>, contents: impl IntoIterator<Item = Content>) -> Self {
        let columns = columns.into();
        let contents = Vec::from_iter(contents);
        Self::HBox { columns, contents }
    }

    /// Creates vertical layout content.
    ///
    /// The `rows` parameter uses CSS Grid syntax to define row sizes.
    /// Common patterns:
    /// - `"auto 1fr auto"` - Header, flexible content, footer
    /// - `"1fr 1fr"` - Equal height rows
    /// - `"100px 1fr"` - Fixed height header, flexible content
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Content;
    ///
    /// let content = Content::vbox("auto 1fr auto", [
    ///     Content::html("<header>Header</header>"),
    ///     Content::from("Main content area"),
    ///     Content::html("<footer>Footer</footer>")
    /// ]);
    /// ```
    pub fn vbox(rows: impl Into<String>, contents: impl IntoIterator<Item = Content>) -> Self {
        let rows = rows.into();
        let contents = Vec::from_iter(contents);
        Self::VBox { rows, contents }
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
            #[cfg(feature = "std")]
            Self::Term { cwd } => {
                write!(fmt, "{}", cwd.display())?;
                Ok(())
            }
            Self::HBox {
                columns: _,
                contents,
            }
            | Self::VBox { rows: _, contents } => {
                let mut first = false;
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
        }
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

    #[test]
    fn test_from_path_html() {
        // Create a temporary HTML file for testing
        let temp_dir = std::env::temp_dir();
        let html_path = temp_dir.join("test.html");
        std::fs::write(&html_path, "<h1>Test HTML</h1>").expect("Failed to write test HTML file");

        let content = Content::from(html_path.as_path());
        match content {
            Content::Html { raw, alt } => {
                assert_eq!(raw, "<h1>Test HTML</h1>");
                assert_eq!(alt, None);
            }
            _ => panic!("Expected HTML content"),
        }

        // Clean up
        let _ = std::fs::remove_file(html_path);
    }

    #[test]
    fn test_from_path_markdown() {
        // Create a temporary Markdown file for testing
        let temp_dir = std::env::temp_dir();
        let md_path = temp_dir.join("test.md");
        let markdown_content = "# Test\nThis is **bold** text.";
        std::fs::write(&md_path, markdown_content).expect("Failed to write test Markdown file");

        let content = Content::from(md_path.as_path());
        match content {
            Content::Html { raw, alt } => {
                assert!(raw.contains("<h1>Test</h1>"));
                assert!(raw.contains("<strong>bold</strong>"));
                assert_eq!(alt, Some(String::from(markdown_content)));
            }
            _ => panic!("Expected HTML content from Markdown"),
        }

        // Clean up
        let _ = std::fs::remove_file(md_path);
    }

    #[test]
    fn test_from_path_text() {
        // Create a temporary text file for testing
        let temp_dir = std::env::temp_dir();
        let txt_path = temp_dir.join("test.txt");
        std::fs::write(&txt_path, "plain text content").expect("Failed to write test text file");

        let content = Content::from(txt_path.as_path());
        match content {
            Content::Text { text } => assert_eq!(text, "plain text content"),
            _ => panic!("Expected Text content"),
        }

        // Clean up
        let _ = std::fs::remove_file(txt_path);
    }
}
