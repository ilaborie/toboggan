use toboggan_core::Content;

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
        return text.to_string();
    }
    idx += 1; // skip '.'

    // Optional: more digits after '.'
    while bytes.get(idx).is_some_and(u8::is_ascii_digit) {
        idx += 1;
    }
    // Must have space after
    if bytes.get(idx) == Some(&b' ') {
        text.get(idx + 1..).unwrap_or("").to_string()
    } else {
        text.to_string()
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
            raw: r#"<div class="step">One</div><div class="step">Two</div>"#.to_string(),
            style: toboggan_core::Style::default(),
            alt: None,
        };
        assert_eq!(count_steps_from_content(&html_with_steps), 2);

        let text_content = Content::Text {
            text: "No steps here".to_string(),
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
