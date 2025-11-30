use toboggan_core::{Content, Slide, SlideKind, Style, Talk};

use crate::error::Result;

// CSS from toboggan-web/src/reset.css
const RESET_CSS: &str = include_str!("../../../toboggan-web/src/reset.css");

// CSS from toboggan-web/src/main.css
const MAIN_CSS: &str = include_str!("../../../toboggan-web/src/main.css");

// CSS from toboggan-web/toboggan-wasm/src/components/slide/style.css
// Adapted to remove :host and shadow DOM specific styles
const SLIDE_CSS: &str =
    include_str!("../../../toboggan-web/toboggan-wasm/src/components/slide/style.css");

// Print CSS for one slide per page
const PRINT_CSS: &str = include_str!("../print.css");

/// Escape HTML special characters
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Render content to HTML string
/// This replicates the logic from toboggan-web/toboggan-wasm/src/utils/dom.rs
fn render_content(content: &Content, wrapper: Option<&str>) -> String {
    let inner = match content {
        Content::Empty => String::new(),
        Content::Text { text } => escape_html(text),
        Content::Html { raw, .. } => raw.clone(),
        Content::Grid { style, cells } => {
            let Style { classes, .. } = style;
            let classes = classes.join(" ");
            let body: String = cells
                .iter()
                .map(|content| render_content(content, None))
                .collect();
            format!(r#"<div class="cell {classes}">{body}</div>"#)
        }
    };

    if let Some(wrapper) = wrapper {
        format!("<{wrapper}>{inner}</{wrapper}>")
    } else {
        inner
    }
}

/// Render a slide to HTML
/// This replicates the logic from toboggan-web/toboggan-wasm/src/components/slide/mod.rs
fn render_slide(slide: &Slide) -> String {
    // Build classes: slide style classes + slide kind class
    let mut classes = slide.style.classes.clone();

    let kind_class = match slide.kind {
        SlideKind::Cover => "cover",
        SlideKind::Part => "part",
        SlideKind::Standard => "standard",
    };
    classes.push(kind_class.to_string());

    let class_string = classes.join(" ");

    // Build inline style attribute if present
    let style_attr = if let Some(style) = &slide.style.style {
        format!(r#" style="{style}""#)
    } else {
        String::new()
    };

    // Render title and body
    let title = render_content(&slide.title, None);
    let body = render_content(&slide.body, Some("article"));

    let content = if title.is_empty() {
        body
    } else {
        format!("<h2>{title}</h2>{body}")
    };

    format!(r#"<section class="{class_string}"{style_attr}>{content}</section>"#)
}

/// Generate a complete static HTML document from a Talk
///
/// # Arguments
///
/// * `talk` - The presentation data
/// * `custom_head_html` - Optional custom HTML to insert at the end of the `<head>` element
#[allow(clippy::unnecessary_wraps)]
pub fn generate_html(talk: &Talk, custom_head_html: Option<&str>) -> Result<Vec<u8>> {
    // Render all slides
    let slides_html =
        talk.slides
            .iter()
            .map(render_slide)
            .fold(String::new(), |mut acc, slide_html| {
                use std::fmt::Write;
                let _ = write!(acc, r#"<div class="toboggan-slide">{slide_html}</div>"#);
                acc
            });

    // Adapt SLIDE_CSS to remove :host selector and adjust for non-shadow-DOM usage
    let adapted_slide_css = SLIDE_CSS
        .replace(":host {", ".toboggan-slide {")
        .replace(":host(", ".toboggan-slide(");

    // Build custom head HTML section if provided
    let custom_head = custom_head_html.map_or(String::new(), |html| format!("    {html}\n"));

    // Build the complete HTML document
    let html = format!(
        r#"<!doctype html>
<html lang="en">

<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{title}</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Bricolage+Grotesque:opsz,wght@12..96,200..800&display=swap" rel="stylesheet">
    <style>
{reset_css}
    </style>
    <style>
{main_css}
    </style>
    <style>
{slide_css}
    </style>
    <style>
{print_css}
    </style>
{custom_head}</head>

<body>
    <main>
{slides_html}
    </main>
</body>

</html>"#,
        title = escape_html(&talk.title),
        reset_css = RESET_CSS,
        main_css = MAIN_CSS,
        slide_css = adapted_slide_css,
        print_css = PRINT_CSS,
        custom_head = custom_head,
        slides_html = slides_html
    );

    Ok(html.into_bytes())
}

#[cfg(test)]
mod tests {
    use toboggan_core::Date;

    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("Hello"), "Hello");
        assert_eq!(
            escape_html("<script>alert('XSS')</script>"),
            "&lt;script&gt;alert(&#39;XSS&#39;)&lt;/script&gt;"
        );
        assert_eq!(escape_html("A & B"), "A &amp; B");
    }

    #[test]
    fn test_render_empty_content() {
        let content = Content::Empty;
        assert_eq!(render_content(&content, None), "");
    }

    #[test]
    fn test_render_text_content() {
        let content = Content::Text {
            text: "Hello <world>".to_string(),
        };
        assert_eq!(render_content(&content, None), "Hello &lt;world&gt;");
    }

    #[test]
    fn test_render_html_content() {
        let content = Content::Html {
            raw: "<p>Hello</p>".to_string(),
            style: Style::default(),
            alt: None,
        };
        assert_eq!(render_content(&content, None), "<p>Hello</p>");
    }

    #[test]
    fn test_render_grid_content() {
        let content = Content::Grid {
            style: Style {
                classes: vec!["test-class".to_string()],
                style: None,
            },
            cells: vec![
                Content::Text {
                    text: "Cell 1".to_string(),
                },
                Content::Text {
                    text: "Cell 2".to_string(),
                },
            ],
        };
        let html = render_content(&content, None);
        assert!(html.contains(r#"<div class="cell test-class">"#));
        assert!(html.contains("Cell 1"));
        assert!(html.contains("Cell 2"));
    }

    #[test]
    fn test_render_content_with_wrapper() {
        let content = Content::Text {
            text: "Hello".to_string(),
        };
        assert_eq!(
            render_content(&content, Some("article")),
            "<article>Hello</article>"
        );
    }

    #[test]
    fn test_generate_html() -> anyhow::Result<()> {
        let mut talk = Talk::new("Test Presentation");
        talk.date = Date::new(2024, 1, 1)?;

        let slide = Slide {
            kind: SlideKind::Cover,
            title: Content::Text {
                text: "Welcome".to_string(),
            },
            body: Content::Html {
                raw: "<p>Hello World</p>".to_string(),
                style: Style::default(),
                alt: None,
            },
            notes: Content::Empty,
            style: Style::default(),
            step_count: 0,
        };
        talk.slides.push(slide);

        let html_bytes = generate_html(&talk, None)?;
        let html = String::from_utf8_lossy(&html_bytes);

        // Check basic structure
        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("<title>Test Presentation</title>"));
        assert!(html.contains(r#"<div class="toboggan-slide">"#));
        assert!(html.contains(r#"<section class="cover""#));
        assert!(html.contains("<h2>Welcome</h2>"));
        assert!(html.contains("<p>Hello World</p>"));

        Ok(())
    }

    #[test]
    fn test_generate_html_with_custom_head() -> anyhow::Result<()> {
        let mut talk = Talk::new("Test");
        talk.date = Date::new(2024, 1, 1)?;

        let custom_html = r#"<meta name="author" content="Test Author">
    <script>console.log('Custom script');</script>"#;

        let html_bytes = generate_html(&talk, Some(custom_html))?;
        let html = String::from_utf8_lossy(&html_bytes);

        // Check custom HTML is present in head
        assert!(html.contains(r#"<meta name="author" content="Test Author">"#));
        assert!(html.contains(r"<script>console.log('Custom script');</script>"));
        // Verify it's before closing head tag
        let head_close_pos = html
            .find("</head>")
            .ok_or_else(|| anyhow::anyhow!("Should have closing head tag"))?;
        let custom_pos = html
            .find("Test Author")
            .ok_or_else(|| anyhow::anyhow!("Should have custom content"))?;
        assert!(custom_pos < head_close_pos, "Custom HTML should be in head");

        Ok(())
    }
}
