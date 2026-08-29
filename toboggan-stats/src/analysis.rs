use toboggan_core::{Content, Slide};

use crate::html::HtmlDocument;

/// Count words in text, stripping markdown link URLs first
#[must_use]
pub fn count_words(text: &str) -> usize {
    let text_without_link_urls = strip_markdown_link_urls(text);
    text_without_link_urls
        .split_whitespace()
        .filter(|word| !word.trim().is_empty())
        .count()
}

/// Remove markdown link URLs, keeping only the link text.
/// Transforms `[text](url)` into `text`.
#[must_use]
pub fn strip_markdown_link_urls(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '[' {
            // Collect link text until ]
            let mut link_text = String::new();
            let mut found_close = false;
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == ']' {
                    found_close = true;
                    break;
                }
                link_text.push(next);
            }
            // Check for (url)
            if found_close && chars.peek() == Some(&'(') {
                chars.next(); // consume '('
                let mut depth = 1;
                for inner in chars.by_ref() {
                    if inner == '(' {
                        depth += 1;
                    } else if inner == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
                result.push_str(&link_text);
            } else {
                result.push('[');
                result.push_str(&link_text);
                if found_close {
                    result.push(']');
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Strip slide counter prefix like "3.5 " or "1. " from text
#[must_use]
pub fn strip_slide_counter(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut idx = 0;

    // Skip leading digits
    while bytes.get(idx).is_some_and(u8::is_ascii_digit) {
        idx += 1;
    }
    // Must have at least one digit and a '.'
    if idx == 0 || bytes.get(idx) != Some(&b'.') {
        return text.to_owned();
    }
    idx += 1; // skip '.'

    // Optional: more digits after '.'
    while bytes.get(idx).is_some_and(u8::is_ascii_digit) {
        idx += 1;
    }
    // Must have space after
    if bytes.get(idx) == Some(&b' ') {
        text.get(idx + 1..).unwrap_or("").to_owned()
    } else {
        text.to_owned()
    }
}

/// Count steps (`.step` CSS class elements) in Content
#[must_use]
pub fn count_steps_from_content(content: &Content) -> usize {
    match content {
        Content::Html { raw, .. } => HtmlDocument::parse_fragment(raw).count_steps(),
        Content::Empty | Content::Text { .. } => 0,
    }
}

/// Extract text from HTML content
#[must_use]
pub fn extract_text_from_html(html: &str) -> String {
    HtmlDocument::parse_fragment(html).extract_text()
}

/// Extract the text of an HTML fragment for a *search* haystack.
///
/// Keeps what [`extract_text_from_html`] drops from a diagram or a figure: a
/// mermaid fence renders to `<svg>`, and the words in it are words on the
/// slide.
#[must_use]
pub fn extract_searchable_text_from_html(html: &str) -> String {
    HtmlDocument::parse_fragment(html).extract_searchable_text()
}

/// Best-effort plain text of a piece of content.
///
/// [`Content::Empty`] is the only `None`: a `Text` holding `""` is content that
/// happens to say nothing, which a caller building a search haystack wants to
/// keep as an empty string rather than as an absent field.
///
/// Unlike [`Content::display_text`], an HTML fragment with no alt text is
/// *parsed* rather than handed back as markup — a search index full of `<p>` is
/// a search index that matches on tag names.
#[must_use]
pub fn content_plain_text(content: &Content) -> Option<String> {
    match content {
        Content::Empty => None,
        Content::Text { text } => Some(text.clone()),
        Content::Html { alt: Some(alt), .. } => Some(alt.clone()),
        Content::Html { raw, .. } => Some(extract_text_from_html(raw)),
    }
}

/// The text a reader would see, extracted from the rendered markup.
///
/// Two things differ from [`content_plain_text`], and both matter to a search
/// index:
///
/// * **The markup, never the alt text.** A slide's body and its notes carry
///   their *Markdown source* as their alt — `# Titre`, `**gras**`,
///   `> citation` (see `HtmlRenderer::render_steps`) — so indexing the alt
///   matches on syntax and quotes syntax back in its snippets. A title is
///   `Content::Text` and has no alt at all, which is why `content_plain_text`
///   can prefer one and this cannot.
/// * **Diagrams and figures are kept**, via
///   [`extract_searchable_text_from_html`]: a mermaid fence renders to `<svg>`,
///   and a slide is often best remembered by a word that appears only in it.
#[must_use]
pub fn rendered_plain_text(content: &Content) -> Option<String> {
    match content {
        Content::Empty => None,
        Content::Text { text } => Some(text.clone()),
        Content::Html { raw, .. } => Some(extract_searchable_text_from_html(raw)),
    }
}

/// The text of a slide's body, as a reader would see it.
#[must_use]
pub fn slide_plain_text(slide: &Slide) -> String {
    rendered_plain_text(&slide.body).unwrap_or_default()
}

/// The text of a slide's speaker notes, as the speaker would read them.
#[must_use]
pub fn notes_plain_text(slide: &Slide) -> String {
    rendered_plain_text(&slide.notes).unwrap_or_default()
}

/// Count images in HTML content
#[must_use]
pub fn count_images_in_html(html: &str) -> usize {
    HtmlDocument::parse_fragment(html).count_images()
}

/// Count list items in HTML content
#[must_use]
pub fn count_list_items_in_html(html: &str) -> usize {
    HtmlDocument::parse_fragment(html).count_list_items()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_counting() {
        assert_eq!(count_words("Hello world"), 2);
        assert_eq!(count_words("  Hello   world  "), 2);
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("One two three four five"), 5);
    }

    #[test]
    fn test_word_counting_with_markdown_links() {
        // Single link - should count only the link text, not the URL
        assert_eq!(count_words("Check [this link](https://example.com)"), 3);

        // Multiple words in link text
        assert_eq!(
            count_words("Visit [my awesome website](https://example.com) today"),
            5
        );

        // Multiple links
        assert_eq!(
            count_words(
                "See [docs](https://docs.example.com) and [source](https://github.com/example)"
            ),
            4
        );

        // Link with complex URL
        assert_eq!(
            count_words(
                "Read [the article](https://example.com/path/to/article?param=value&other=123)"
            ),
            3
        );

        // Text without links should work as before
        assert_eq!(count_words("No links here just text"), 5);

        // Mixed content
        assert_eq!(
            count_words(
                "Start text [link one](https://url1.com) middle [link two](https://url2.com) end"
            ),
            8
        );
    }

    #[test]
    fn test_strip_slide_counter() {
        assert_eq!(strip_slide_counter("3.5 Diagram"), "Diagram");
        assert_eq!(strip_slide_counter("1. Introduction"), "Introduction");
        assert_eq!(strip_slide_counter("Diagram"), "Diagram");
        assert_eq!(strip_slide_counter("10.20 Test"), "Test");
        assert_eq!(strip_slide_counter(""), "");
    }

    #[test]
    fn test_count_steps_from_content() {
        let html_with_steps = Content::Html {
            raw: r#"<div class="step">One</div><div class="step">Two</div>"#.to_owned(),
            style: toboggan_core::Style::default(),
            alt: None,
        };
        assert_eq!(count_steps_from_content(&html_with_steps), 2);

        let text_content = Content::Text {
            text: "No steps here".to_owned(),
        };
        assert_eq!(count_steps_from_content(&text_content), 0);

        let empty_content = Content::Empty;
        assert_eq!(count_steps_from_content(&empty_content), 0);
    }

    #[test]
    fn test_extract_text_from_html() {
        let html = "<p>Hello</p><p>World</p>";
        assert_eq!(extract_text_from_html(html), "Hello World");
    }

    /// The alt text is what the slide says it means; the markup is the fallback,
    /// and it is parsed rather than passed through — a haystack full of `<p>`
    /// matches on tag names.
    #[test]
    fn content_plain_text_prefers_alt_then_parses_markup() {
        let with_alt = Content::html_with_alt("<p>Bonjour <b>tout</b></p>", "Bonjour tout");
        assert_eq!(
            content_plain_text(&with_alt).as_deref(),
            Some("Bonjour tout")
        );

        let without_alt = Content::html("<p>Bonjour <b>tout</b></p>");
        assert_eq!(
            content_plain_text(&without_alt).as_deref(),
            Some("Bonjour tout")
        );
    }

    /// Only `Empty` is absent: a `Text` holding `""` is content that happens to
    /// say nothing, which a search haystack wants as an empty string.
    #[test]
    fn only_empty_content_has_no_plain_text() {
        assert_eq!(content_plain_text(&Content::Empty), None);
        assert_eq!(content_plain_text(&Content::text("")).as_deref(), Some(""));
    }

    /// The body and the notes carry their Markdown source as their alt text, so
    /// reading the alt would index `#` and `**` and quote them back in a
    /// snippet. The markup is what a reader sees.
    #[test]
    fn a_slide_yields_the_rendered_text_of_its_body_and_its_notes() {
        let slide = Slide {
            body: Content::html_with_alt("<p>On the <b>projector</b></p>", "On the **projector**"),
            notes: Content::html_with_alt("<p>For the speaker</p>", "For the speaker"),
            ..Default::default()
        };
        assert_eq!(slide_plain_text(&slide), "On the projector");
        assert_eq!(notes_plain_text(&slide), "For the speaker");

        let bare = Slide::default();
        assert_eq!(slide_plain_text(&bare), "");
        assert_eq!(notes_plain_text(&bare), "");
    }

    #[test]
    fn test_count_images_in_html() {
        let html = r#"<img src="a.jpg"><svg></svg><figure></figure>"#;
        assert_eq!(count_images_in_html(html), 3);
    }

    #[test]
    fn test_count_list_items_in_html() {
        let html = "<ul><li>A</li><li>B</li></ul>";
        assert_eq!(count_list_items_in_html(html), 2);
    }
}
