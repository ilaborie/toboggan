use std::path::PathBuf;

use crate::parser::CssClasses;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum CommentType {
    Pause(CssClasses),
    Notes,
    Code { info: String, path: PathBuf },
    Unknown,
}

const MARKER_PAUSE: &str = "pause";
const MARKER_NOTES: &str = "notes";
const MARKER_CODE: &str = "code";

fn parse_comment(html: &str) -> Option<&str> {
    let html = html.trim();
    if !html.starts_with("<!--") || !html.ends_with("-->") {
        return None;
    }
    let html = html
        .trim_start_matches("<!--")
        .trim_end_matches("-->")
        .trim();
    Some(html)
}

pub(super) fn parse_comment_content(html: &str) -> CommentType {
    let Some(comment_content) = parse_comment(html) else {
        return CommentType::Unknown;
    };

    if comment_content.starts_with(MARKER_PAUSE) {
        let classes_str = comment_content.trim_start_matches(MARKER_PAUSE);
        let classes = parse_classes(classes_str);
        CommentType::Pause(classes)
    } else if comment_content.to_lowercase().starts_with(MARKER_NOTES) {
        CommentType::Notes
    } else if comment_content.starts_with(MARKER_CODE) {
        parse_code_comment(comment_content)
    } else {
        CommentType::Unknown
    }
}

fn parse_classes(html: &str) -> CssClasses {
    let trimmed = html.trim();
    if !trimmed.starts_with(':') {
        return CssClasses::default();
    }
    trimmed
        .trim_start_matches(':')
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

fn parse_code_comment(comment_content: &str) -> CommentType {
    let content_after_code = comment_content.trim_start_matches(MARKER_CODE).trim();

    // Remove leading ':' if present
    let content_after_code = content_after_code
        .strip_prefix(':')
        .unwrap_or(content_after_code);

    // Split on ':' to get info and path parts
    let parts: Vec<&str> = content_after_code.splitn(2, ':').collect();

    if let (Some(info_part), Some(path_part)) = (parts.first(), parts.get(1)) {
        let info = info_part.trim().to_string();
        let path = PathBuf::from(path_part.trim());
        CommentType::Code { info, path }
    } else {
        // If we can't parse properly, fall back to Unknown
        CommentType::Unknown
    }
}

pub(super) fn parse_pause(html: &str) -> Option<CssClasses> {
    match parse_comment_content(html) {
        CommentType::Pause(classes) => Some(classes),
        _ => None,
    }
}

pub(super) fn is_notes(html: &str) -> bool {
    matches!(parse_comment_content(html), CommentType::Notes)
}

pub(super) fn parse_code(html: &str) -> Option<(String, PathBuf)> {
    match parse_comment_content(html) {
        CommentType::Code { info, path } => Some((info, path)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_type_parsing() {
        // Test pause comment
        let pause_comment = "<!-- pause :highlight -->";
        if let CommentType::Pause(classes) = parse_comment_content(pause_comment) {
            assert_eq!(classes, vec!["highlight".to_string()]);
        } else {
            panic!("Expected Pause variant");
        }

        // Test notes comment
        let notes_comment = "<!-- notes -->";
        assert_eq!(parse_comment_content(notes_comment), CommentType::Notes);

        // Test notes comment with different case
        let notes_comment_upper = "<!-- NOTES -->";
        assert_eq!(
            parse_comment_content(notes_comment_upper),
            CommentType::Notes
        );

        // Test code comment
        let code_comment = "<!-- code:rust:src/main.rs -->";
        if let CommentType::Code { info, path } = parse_comment_content(code_comment) {
            assert_eq!(info, "rust");
            assert_eq!(path, PathBuf::from("src/main.rs"));
        } else {
            panic!("Expected Code variant");
        }

        // Test code comment with spaces
        let code_comment_spaces = "<!-- code : javascript : app.js -->";
        if let CommentType::Code { info, path } = parse_comment_content(code_comment_spaces) {
            assert_eq!(info, "javascript");
            assert_eq!(path, PathBuf::from("app.js"));
        } else {
            panic!("Expected Code variant with spaces");
        }

        // Test malformed code comment (missing path)
        let malformed_code_comment = "<!-- code:rust -->";
        assert_eq!(
            parse_comment_content(malformed_code_comment),
            CommentType::Unknown
        );

        // Test unknown comment
        let unknown_comment = "<!-- random comment -->";
        assert_eq!(parse_comment_content(unknown_comment), CommentType::Unknown);

        // Test non-comment
        let not_comment = "regular text";
        assert_eq!(parse_comment_content(not_comment), CommentType::Unknown);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn test_backward_compatibility() {
        // Test that legacy functions still work
        let pause_comment = "<!-- pause :class1 class2 -->";
        let classes = parse_pause(pause_comment).expect("a pause");
        assert_eq!(classes, vec!["class1".to_string(), "class2".to_string()]);

        let notes_comment = "<!-- notes -->";
        assert!(is_notes(notes_comment));

        let code_comment = "<!-- code:python:script.py -->";
        let (info, path) = parse_code(code_comment).expect("a code");
        assert_eq!(info, "python");
        assert_eq!(path, PathBuf::from("script.py"));
    }
}
