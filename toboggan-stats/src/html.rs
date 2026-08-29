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

/// Tags dropped when building a *search* haystack.
///
/// Narrower than [`EXCLUDED_TAGS`] on purpose: `style` and `script` are the only
/// two whose text a reader never sees. A diagram's node labels and a figure's
/// caption are words on the slide, and a speaker hunting for "the one with the
/// retry loop in it" is searching for exactly the kind of word that only ever
/// appears inside a `<svg>` a mermaid fence produced.
const UNSEARCHABLE_TAGS: &[&str] = &["style", "script"];

/// Tags excluded on top of [`EXCLUDED_TAGS`] when counting *spoken* words.
///
/// Only `pre`, never `code`: comrak renders a fenced or indented block as
/// `<pre><code>`, so `pre` covers block code, while an inline `` `code` `` span
/// is a bare `<code>` — and an identifier written inline is read out loud.
const UNSPOKEN_TAGS: &[&str] = &["style", "script", "svg", "figure", "pre"];

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
    ///
    /// This is the *whole* readable text of the fragment, code blocks included,
    /// and it is what the slide-overview search index is built from — searching
    /// a deck for a snippet it visibly contains has to find it. Word counts use
    /// [`Self::extract_spoken_text`] instead.
    #[must_use]
    pub fn extract_text(&self) -> String {
        self.extract_text_excluding(EXCLUDED_TAGS)
    }

    /// Extract the text a search should look in, keeping diagrams and figures.
    ///
    /// Wider than [`Self::extract_text`]: this drops only `style` and `script`,
    /// so a mermaid diagram's labels and a figure's caption are searchable. Use
    /// it for a haystack, not for a word count — [`Self::extract_spoken_text`]
    /// is what the duration estimate is built on, and it excludes both.
    #[must_use]
    pub fn extract_searchable_text(&self) -> String {
        self.extract_text_excluding(UNSEARCHABLE_TAGS)
    }

    /// Extract only the text a speaker actually says, dropping block code.
    ///
    /// Word counts drive the duration estimate, and nobody reads a fenced block
    /// out word by word — counting it made a code-heavy deck look far longer
    /// than it is.
    #[must_use]
    pub fn extract_spoken_text(&self) -> String {
        self.extract_text_excluding(UNSPOKEN_TAGS)
    }

    fn extract_text_excluding(&self, excluded: &[&str]) -> String {
        let mut result = String::new();
        for element in self.document.root_element().children() {
            if let Some(element_ref) = ElementRef::wrap(element) {
                Self::extract_text_recursive(element_ref, excluded, &mut result);
            } else if let Some(text) = element.value().as_text() {
                Self::append_text(&mut result, text.trim());
            }
        }
        result
    }

    fn extract_text_recursive(element: ElementRef<'_>, excluded: &[&str], result: &mut String) {
        let tag_name = element.value().name();

        // Skip excluded tags entirely
        if excluded.contains(&tag_name) {
            return;
        }

        for child in element.children() {
            if let Some(child_element) = ElementRef::wrap(child) {
                Self::extract_text_recursive(child_element, excluded, result);
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

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod query_tests {
    use super::*;

    /// These five are the parser behind `html/raw-script`, `html/heading-h1`,
    /// `link/broken`, `html/img-missing-alt` and both `code/*` rules, and were
    /// covered only through `toboggan-lint` — so a change here surfaced as a
    /// lint failure in another crate, pointing at the rule rather than the
    /// parse.
    #[test]
    fn a_raw_script_is_found_anywhere_in_the_fragment() {
        assert!(HtmlDocument::parse_fragment("<p>hi</p><script>x()</script>").has_script());
        assert!(
            HtmlDocument::parse_fragment("<div><script src=\"a.js\"></script></div>").has_script()
        );
        assert!(!HtmlDocument::parse_fragment("<p>no script here</p>").has_script());
    }

    #[test]
    fn an_h1_in_the_body_is_found() {
        assert!(HtmlDocument::parse_fragment("<h1>Title</h1>").has_h1());
        assert!(!HtmlDocument::parse_fragment("<h2>Subtitle</h2>").has_h1());
    }

    #[test]
    fn image_sources_come_back_in_document_order() {
        let document = HtmlDocument::parse_fragment(
            r#"<img src="one.png"><p>x</p><img src="two.png" alt="a">"#,
        );
        assert_eq!(document.image_sources(), vec!["one.png", "two.png"]);
        // An `<img>` with no `src` has nothing to check and is not reported.
        assert!(
            HtmlDocument::parse_fragment("<img alt=\"a\">")
                .image_sources()
                .is_empty()
        );
    }

    #[test]
    fn link_targets_come_back_in_document_order() {
        let document =
            HtmlDocument::parse_fragment(r#"<a href="/a">A</a><a>no href</a><a href="/b">B</a>"#);
        assert_eq!(document.link_targets(), vec!["/a", "/b"]);
    }

    /// The language comes off comrak's `language-*` class, and the line count
    /// is what `code/too-long` compares against a budget.
    #[test]
    fn a_fenced_block_carries_its_language_and_line_count() {
        let document = HtmlDocument::parse_fragment(
            "<pre><code class=\"language-rust\">fn main() {\nprintln!(\"hi\");\n}\n</code></pre>",
        );
        assert_eq!(
            document.code_blocks(),
            vec![CodeBlock {
                language: Some("rust".to_owned()),
                lines: 3,
            }]
        );
    }

    /// A block with no language is what `code/no-language` reports, so the
    /// distinction between `None` and `Some("")` is load-bearing.
    #[test]
    fn a_block_without_a_language_reports_none() {
        let document = HtmlDocument::parse_fragment("<pre><code>plain\ntext\n</code></pre>");
        assert_eq!(
            document
                .code_blocks()
                .first()
                .map(|block| block.language.clone()),
            Some(None)
        );
    }

    /// Only the trailing newline is ignored; a block that does not end in one
    /// still counts its last line. The `lines: 1` case is the boundary
    /// `code/too-long` never sees but every short block hits.
    #[test]
    fn a_trailing_newline_does_not_add_a_line() {
        let with = HtmlDocument::parse_fragment("<pre><code>one\n</code></pre>");
        let without = HtmlDocument::parse_fragment("<pre><code>one</code></pre>");
        assert_eq!(with.code_blocks()[0].lines, 1);
        assert_eq!(without.code_blocks()[0].lines, 1);
    }

    #[test]
    fn several_blocks_are_all_returned() {
        let document = HtmlDocument::parse_fragment(
            "<pre><code class=\"language-sh\">ls</code></pre><p>x</p><pre><code>plain</code></pre>",
        );
        let blocks = document.code_blocks();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language.as_deref(), Some("sh"));
        assert_eq!(blocks[1].language, None);
    }
}

/// The split between "all the text" and "the text someone says out loud".
///
/// These two extractions feed different consumers — the slide-overview search
/// index reads [`HtmlDocument::extract_text`], word counts read
/// [`HtmlDocument::extract_spoken_text`] — so collapsing them back into one
/// would either make code unsearchable or make code-heavy decks read as far
/// longer than they are. Pinned here so that cannot happen quietly.
#[cfg(test)]
mod spoken_text_tests {
    use super::*;

    const FENCED: &str =
        "<p>Run this</p><pre><code class=\"language-sh\">cargo build --release</code></pre>";

    #[test]
    fn block_code_is_searchable_but_not_spoken() {
        let document = HtmlDocument::parse_fragment(FENCED);

        let searchable = document.extract_text();
        assert!(
            searchable.contains("cargo build --release"),
            "the search index must still find code: {searchable:?}"
        );

        let spoken = document.extract_spoken_text();
        assert_eq!(spoken, "Run this");
    }

    #[test]
    fn inline_code_is_still_spoken() {
        let spoken = HtmlDocument::parse_fragment("<p>Set <code>hidden_in</code> to skip it</p>")
            .extract_spoken_text();

        assert_eq!(spoken, "Set hidden_in to skip it");
    }

    #[test]
    fn a_style_block_is_in_neither() {
        let document =
            HtmlDocument::parse_fragment("<style>.card { color: red; }</style><p>Cards</p>");

        assert_eq!(document.extract_text(), "Cards");
        assert_eq!(document.extract_spoken_text(), "Cards");
    }
}
