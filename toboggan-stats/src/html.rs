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

/// Pre-compiled selector for nested steps (`.step` inside another `.step`)
#[allow(clippy::expect_used)]
static NESTED_STEP_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".step .step").expect("nested step selector should be valid"));

/// Pre-compiled selector for `script` elements
#[allow(clippy::expect_used)]
static SCRIPT_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("script").expect("script selector should be valid"));

/// Pre-compiled selector for `h1` elements
#[allow(clippy::expect_used)]
static H1_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h1").expect("h1 selector should be valid"));

/// Pre-compiled selector for code blocks (`<code>` inside a `<pre>`)
#[allow(clippy::expect_used)]
static CODE_BLOCK_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("pre code").expect("code block selector should be valid"));

/// Pre-compiled selector for `a` elements
#[allow(clippy::expect_used)]
static ANCHOR_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("a").expect("anchor selector should be valid"));

/// Tags whose content should be excluded from text extraction
const EXCLUDED_TAGS: &[&str] = &["style", "script", "svg", "figure"];

/// Prefix comrak puts on a fenced block's language class.
const LANGUAGE_CLASS_PREFIX: &str = "language-";

/// A fenced or indented code block found in slide content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    /// The fence's language, from the `language-*` class comrak emits.
    /// `None` for a block written without one (or an indented block, which
    /// cannot carry a language at all).
    pub language: Option<String>,
    /// Lines of code, ignoring a trailing newline.
    pub lines: usize,
}

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

    /// Whether the fragment contains a raw `<script>` element.
    ///
    /// Asks the parsed tree rather than searching the source for `<script`,
    /// which also matched the text of an HTML comment and forced a full
    /// lowercased copy of the body to be allocated for every check.
    #[must_use]
    pub fn has_script(&self) -> bool {
        self.document.select(&SCRIPT_SELECTOR).next().is_some()
    }

    /// Whether the fragment contains an `<h1>` element.
    #[must_use]
    pub fn has_h1(&self) -> bool {
        self.document.select(&H1_SELECTOR).next().is_some()
    }

    /// Count `.step` elements nested inside another `.step`.
    ///
    /// Nested steps break the frontend reveal logic, so any non-zero count is a
    /// lint error.
    #[must_use]
    pub fn count_nested_steps(&self) -> usize {
        self.document.select(&NESTED_STEP_SELECTOR).count()
    }

    /// Count `.step` elements with neither text content nor child elements.
    ///
    /// A genuinely empty step (e.g. a stray `<!-- pause -->`) reveals nothing.
    #[must_use]
    pub fn count_empty_steps(&self) -> usize {
        self.document
            .select(&STEP_SELECTOR)
            .filter(|step| {
                let no_text = step.text().collect::<String>().trim().is_empty();
                let no_children = step.children().find_map(ElementRef::wrap).is_none();
                no_text && no_children
            })
            .count()
    }

    /// Count `<img>` elements that are missing a non-empty `alt` attribute.
    ///
    /// Missing alt text hurts accessibility and the Typst/PDF export.
    #[must_use]
    pub fn count_images_without_alt(&self) -> usize {
        self.document
            .select(&IMG_SELECTOR)
            .filter(|img| {
                img.value()
                    .attr("alt")
                    .is_none_or(|alt| alt.trim().is_empty())
            })
            .count()
    }

    /// Every `src` an `<img>` points at, in document order.
    #[must_use]
    pub fn image_sources(&self) -> Vec<&str> {
        self.document
            .select(&IMG_SELECTOR)
            .filter_map(|img| img.value().attr("src"))
            .collect()
    }

    /// Every `href` an `<a>` points at, in document order.
    #[must_use]
    pub fn link_targets(&self) -> Vec<&str> {
        self.document
            .select(&ANCHOR_SELECTOR)
            .filter_map(|anchor| anchor.value().attr("href"))
            .collect()
    }

    /// Every code block in the fragment, with its language and line count.
    ///
    /// One pass serving both code rules, so a slide's body is walked once
    /// rather than once per question asked about it.
    #[must_use]
    pub fn code_blocks(&self) -> Vec<CodeBlock> {
        self.document
            .select(&CODE_BLOCK_SELECTOR)
            .map(|block| {
                let language = block
                    .value()
                    .classes()
                    .find_map(|class| class.strip_prefix(LANGUAGE_CLASS_PREFIX))
                    .map(str::to_owned);
                // `text()` concatenates the highlighter's per-token spans back
                // into the original source, so the newlines survive.
                let source = block.text().collect::<String>();
                CodeBlock {
                    language,
                    lines: source.trim_end_matches('\n').lines().count(),
                }
            })
            .collect()
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

    #[test]
    fn test_count_nested_steps() {
        let html = r#"<div class="step">outer<div class="step">inner</div></div><div class="step">flat</div>"#;
        let doc = HtmlDocument::parse_fragment(html);
        assert_eq!(doc.count_nested_steps(), 1);
    }

    #[test]
    fn test_count_empty_steps() {
        let html = r#"<div class="step"></div><div class="step">  </div><div class="step">content</div><div class="step"><img src="a.png"></div>"#;
        let doc = HtmlDocument::parse_fragment(html);
        // First two are empty; the content one and the image one are not.
        assert_eq!(doc.count_empty_steps(), 2);
    }

    #[test]
    fn test_count_images_without_alt() {
        let html = r#"<img src="a.png"><img src="b.png" alt="">< img src="c.png" alt="ok">"#;
        let doc = HtmlDocument::parse_fragment(html);
        // `a` has no alt, `b` has empty alt; the malformed `< img` is not parsed as img.
        assert_eq!(doc.count_images_without_alt(), 2);
    }
}
