use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};

/// Pre-compiled selector for `.step` class
#[allow(clippy::expect_used)]
static STEP_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".step").expect("step selector should be valid"));

/// Pre-compiled selector for `img` elements
#[allow(clippy::expect_used)]
static IMG_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("img").expect("img selector should be valid"));

/// Pre-compiled selector for `svg` elements
#[allow(clippy::expect_used)]
static SVG_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("svg").expect("svg selector should be valid"));

/// Pre-compiled selector for `figure` elements
#[allow(clippy::expect_used)]
static FIGURE_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("figure").expect("figure selector should be valid"));

/// Pre-compiled selector for `li` elements
#[allow(clippy::expect_used)]
static LIST_ITEM_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("li").expect("li selector should be valid"));

/// Tags whose content should be excluded from text extraction
const EXCLUDED_TAGS: &[&str] = &["style", "script", "svg", "figure"];

/// Wrapper around `scraper::Html` for convenient HTML querying
#[derive(Debug)]
pub struct HtmlDocument {
    document: Html,
}

impl HtmlDocument {
    /// Parse an HTML fragment (not a full document)
    #[must_use]
    pub fn parse_fragment(html: &str) -> Self {
        Self {
            document: Html::parse_fragment(html),
        }
    }

    /// Count elements matching the `.step` CSS class
    #[must_use]
    pub fn count_steps(&self) -> usize {
        self.document.select(&STEP_SELECTOR).count()
    }

    /// Count all image elements (`img`, `svg`, `figure`)
    #[must_use]
    pub fn count_images(&self) -> usize {
        let img_count = self.document.select(&IMG_SELECTOR).count();
        let svg_count = self.document.select(&SVG_SELECTOR).count();
        let figure_count = self.document.select(&FIGURE_SELECTOR).count();
        img_count + svg_count + figure_count
    }

    /// Count list items (`li` elements)
    #[must_use]
    pub fn count_list_items(&self) -> usize {
        self.document.select(&LIST_ITEM_SELECTOR).count()
    }

    /// Extract text content, excluding content from style, script, svg, and figure tags
    #[must_use]
    pub fn extract_text(&self) -> String {
        let mut result = String::new();
        for element in self.document.root_element().children() {
            if let Some(element_ref) = ElementRef::wrap(element) {
                Self::extract_text_recursive(element_ref, &mut result);
            } else if let Some(text) = element.value().as_text() {
                Self::append_text(&mut result, text.trim());
            }
        }
        result
    }

    fn extract_text_recursive(element: ElementRef<'_>, result: &mut String) {
        let tag_name = element.value().name();

        // Skip excluded tags entirely
        if EXCLUDED_TAGS.contains(&tag_name) {
            return;
        }

        for child in element.children() {
            if let Some(child_element) = ElementRef::wrap(child) {
                Self::extract_text_recursive(child_element, result);
            } else if let Some(text) = child.value().as_text() {
                Self::append_text(result, text.trim());
            }
        }
    }

    fn append_text(result: &mut String, text: &str) {
        if text.is_empty() {
            return;
        }
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_steps() {
        let html = r#"
            <div class="step step-0">First step</div>
            <div class="step step-1 highlight">Second step</div>
            <div class="step step-2">Third step</div>
        "#;
        let doc = HtmlDocument::parse_fragment(html);
        assert_eq!(doc.count_steps(), 3);
    }

    #[test]
    fn test_count_steps_empty() {
        let html = "<p>No steps here</p>";
        let doc = HtmlDocument::parse_fragment(html);
        assert_eq!(doc.count_steps(), 0);
    }

    #[test]
    fn test_count_images_img() {
        let html = r#"<p>Text <img src="a.jpg"> more <img src="b.png"></p>"#;
        let doc = HtmlDocument::parse_fragment(html);
        assert_eq!(doc.count_images(), 2);
    }

    #[test]
    fn test_count_images_svg() {
        let html = r#"<div><svg width="100"><circle/></svg></div>"#;
        let doc = HtmlDocument::parse_fragment(html);
        assert_eq!(doc.count_images(), 1);
    }

    #[test]
    fn test_count_images_figure() {
        let html = r#"<figure><img src="test.jpg"><figcaption>Caption</figcaption></figure>"#;
        let doc = HtmlDocument::parse_fragment(html);
        // Counts both figure and the img inside
        assert_eq!(doc.count_images(), 2);
    }

    #[test]
    fn test_count_images_mixed() {
        let html = r#"
            <img src="photo.jpg">
            <svg><path d="M0,0"/></svg>
            <figure><img src="chart.png"><figcaption>Chart</figcaption></figure>
            <svg width="50"><rect/></svg>
        "#;
        let doc = HtmlDocument::parse_fragment(html);
        // 1 img + 2 svg + 1 figure + 1 img inside figure = 5
        assert_eq!(doc.count_images(), 5);
    }

    #[test]
    fn test_count_list_items() {
        let html = r"<ul><li>Item 1</li><li>Item 2</li><li>Item 3</li></ul>";
        let doc = HtmlDocument::parse_fragment(html);
        assert_eq!(doc.count_list_items(), 3);
    }

    #[test]
    fn test_count_list_items_nested() {
        let html =
            r"<ul><li>Outer 1<ul><li>Inner 1</li><li>Inner 2</li></ul></li><li>Outer 2</li></ul>";
        let doc = HtmlDocument::parse_fragment(html);
        assert_eq!(doc.count_list_items(), 4);
    }

    #[test]
    fn test_extract_text_simple() {
        let html = "<p>Hello world</p>";
        let doc = HtmlDocument::parse_fragment(html);
        assert_eq!(doc.extract_text(), "Hello world");
    }

    #[test]
    fn test_extract_text_excludes_style() {
        let html = r"<p>Text</p><style>body { color: red; }</style><p>More</p>";
        let doc = HtmlDocument::parse_fragment(html);
        let text = doc.extract_text();
        assert!(text.contains("Text"));
        assert!(text.contains("More"));
        assert!(!text.contains("color"));
        assert!(!text.contains("red"));
    }

    #[test]
    fn test_extract_text_excludes_script() {
        let html = r#"<p>Content</p><script>console.log("test");</script><p>End</p>"#;
        let doc = HtmlDocument::parse_fragment(html);
        let text = doc.extract_text();
        assert!(text.contains("Content"));
        assert!(text.contains("End"));
        assert!(!text.contains("console"));
        assert!(!text.contains("log"));
    }

    #[test]
    fn test_extract_text_excludes_svg() {
        let html =
            r#"<div>Text</div><svg><path d="M0,0"/><text>SVG Text</text></svg><div>More</div>"#;
        let doc = HtmlDocument::parse_fragment(html);
        let text = doc.extract_text();
        assert!(text.contains("Text"));
        assert!(text.contains("More"));
        assert!(!text.contains("SVG Text"));
        assert!(!text.contains("M0,0"));
    }

    #[test]
    fn test_extract_text_excludes_figure() {
        let html = r#"<p>Main</p><figure><img src="test.jpg"><figcaption>Caption text</figcaption></figure><p>End</p>"#;
        let doc = HtmlDocument::parse_fragment(html);
        let text = doc.extract_text();
        assert!(text.contains("Main"));
        assert!(text.contains("End"));
        assert!(!text.contains("Caption text"));
    }

    #[test]
    fn test_extract_text_multiple_spaces() {
        let html = "<p>  Hello  </p>  <p>  World  </p>";
        let doc = HtmlDocument::parse_fragment(html);
        let text = doc.extract_text();
        assert_eq!(text, "Hello World");
    }
}
