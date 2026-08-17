use toboggan_core::{Content, Slide, SlideKind, Talk};

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

// Keyboard navigation for the exported file. Inlined, like the stylesheets: an
// export is handed around as a single file and shown from a lectern with no
// guarantee of a network.
const NAVIGATE_JS: &str = include_str!("../navigate.js");

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
    classes.push(kind_class.to_owned());

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
pub(super) fn generate_html(talk: &Talk, custom_head_html: Option<&str>) -> Result<Vec<u8>> {
    // Render all slides. Each carries an `id`, so `deck.html#slide-12` opens on
    // that slide — with the navigator running, and by scrolling to it without.
    let slides_html =
        talk.slides
            .iter()
            .enumerate()
            .fold(String::new(), |mut acc, (index, slide)| {
                use std::fmt::Write;
                let number = index + 1;
                let slide_html = render_slide(slide);
                let _ = write!(
                    acc,
                    r#"<div class="toboggan-slide" id="slide-{number}">{slide_html}</div>"#
                );
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
    <script>
{navigate_js}
    </script>
</body>

</html>"#,
        title = escape_html(&talk.title),
        reset_css = RESET_CSS,
        main_css = MAIN_CSS,
        slide_css = adapted_slide_css,
        print_css = PRINT_CSS,
        custom_head = custom_head,
        slides_html = slides_html,
        navigate_js = NAVIGATE_JS
    );

    Ok(html.into_bytes())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_core::{Date, Style};

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
            text: "Hello <world>".to_owned(),
        };
        assert_eq!(render_content(&content, None), "Hello &lt;world&gt;");
    }

    #[test]
    fn test_render_html_content() {
        let content = Content::Html {
            raw: "<p>Hello</p>".to_owned(),
            style: Style::default(),
            alt: None,
        };
        assert_eq!(render_content(&content, None), "<p>Hello</p>");
    }

    #[test]
    fn test_render_content_with_wrapper() {
        let content = Content::Text {
            text: "Hello".to_owned(),
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
                text: "Welcome".to_owned(),
            },
            body: Content::Html {
                raw: "<p>Hello World</p>".to_owned(),
                style: Style::default(),
                alt: None,
            },
            notes: Content::Empty,
            style: Style::default(),
            terminals: Vec::new(),
            ..Default::default()
        };
        talk.slides.push(slide);

        let html_bytes = generate_html(&talk, None)?;
        let html = String::from_utf8_lossy(&html_bytes);

        // Check basic structure
        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("<title>Test Presentation</title>"));
        assert!(html.contains(r#"<div class="toboggan-slide" id="slide-1">"#));
        assert!(html.contains(r#"<section class="cover""#));
        assert!(html.contains("<h2>Welcome</h2>"));
        assert!(html.contains("<p>Hello World</p>"));

        // An exported deck is handed around as a file and shown from a lectern
        // with no guarantee of network, so it must not fetch anything
        // off-origin: no CDN, no web font, no script. Inline `data:` SVGs carry
        // an `xmlns="http://www.w3.org/..."` namespace, which names a spec
        // rather than something the browser requests, so match on the
        // attributes that actually cause a fetch.
        for attribute in [r#"href="http"#, r#"src="http"#, "url(http", "url(\"http"] {
            assert!(
                !html.contains(attribute),
                "exported HTML must be self-contained, found `{attribute}`"
            );
        }

        Ok(())
    }

    /// The export is a file someone opens from a lectern, so the navigator has
    /// to be in it — not fetched — and every slide has to be addressable.
    #[test]
    fn the_export_carries_its_own_navigation() -> anyhow::Result<()> {
        let mut talk = Talk::new("Test");
        talk.slides.push(Slide::new("One"));
        talk.slides.push(Slide::new("Two"));

        let html = String::from_utf8(generate_html(&talk, None)?)?;

        assert!(html.contains("<script>"), "the navigator ships inline");
        assert!(html.contains(r#"id="slide-1""#));
        assert!(html.contains(r#"id="slide-2""#));
        // Nothing may run before the slides exist: the script reads them on the
        // way in, and mounting it in <head> would find an empty document.
        let script = html.find("<script>").expect("script");
        let last_slide = html.find(r#"id="slide-2""#).expect("slide");
        assert!(last_slide < script, "the navigator comes after the slides");

        Ok(())
    }

    /// Presentation mode is opt-in from the script and screen-only, so a deck
    /// opened with scripting off — and the PDF export, which renders this same
    /// document — still get every slide and every step.
    #[test]
    fn presentation_mode_never_reaches_print() -> anyhow::Result<()> {
        let talk = Talk::new("Test");
        let html = String::from_utf8(generate_html(&talk, None)?)?;

        let screen_only = html
            .find("@media screen")
            .expect("presentation mode is screen-only");
        let gate = html
            .find("html.toboggan-js")
            .expect("presentation mode is gated on the script having run");
        assert!(screen_only < gate);

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

        // The custom head comes last so a deck can override the bundled styles.
        let styles_pos = html
            .find("<style>")
            .ok_or_else(|| anyhow::anyhow!("Should have bundled styles"))?;
        assert!(
            styles_pos < custom_pos,
            "custom head must come after the bundled styles so a deck can override them"
        );

        Ok(())
    }
}
