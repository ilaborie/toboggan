use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Display;
#[cfg(feature = "std")]
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    #[default]
    Empty,
    Text {
        text: String,
    },
    Html {
        raw: String,
        alt: Option<String>,
    },
    IFrame {
        url: String,
    },
    #[cfg(feature = "std")]
    Term {
        cwd: PathBuf,
    },
    HBox {
        columns: String,
        contents: Vec<Content>,
    },
    VBox {
        rows: String,
        contents: Vec<Content>,
    },
}

impl Content {
    pub fn text(text: impl Into<String>) -> Self {
        let text = text.into();
        Self::Text { text }
    }

    pub fn html(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let alt = None;
        Self::Html { raw, alt }
    }

    pub fn html_with_alt(raw: impl Into<String>, alt: impl Into<String>) -> Self {
        let raw = raw.into();
        let alt = Some(alt.into());
        Self::Html { raw, alt }
    }

    pub fn iframe(url: impl Into<String>) -> Self {
        let url = url.into();
        Self::IFrame { url }
    }

    #[cfg(feature = "std")]
    pub fn term(cwd: impl Into<PathBuf>) -> Self {
        let cwd = cwd.into();
        Self::Term { cwd }
    }

    pub fn hbox(columns: impl Into<String>, contents: impl IntoIterator<Item = Content>) -> Self {
        let columns = columns.into();
        let contents = Vec::from_iter(contents);
        Self::HBox { columns, contents }
    }

    pub fn vbox(rows: impl Into<String>, contents: impl IntoIterator<Item = Content>) -> Self {
        let rows = rows.into();
        let contents = Vec::from_iter(contents);
        Self::VBox { rows, contents }
    }
}

impl From<&str> for Content {
    fn from(text: &str) -> Self {
        Self::text(text)
    }
}

#[cfg(feature = "std")]
impl From<&Path> for Content {
    fn from(path: &Path) -> Self {
        use pulldown_cmark::{Options, Parser, html};

        // Read the file content, panic if it fails
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read file {}: {}", path.display(), e));

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
