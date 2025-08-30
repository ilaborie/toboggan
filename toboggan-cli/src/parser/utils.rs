//! Shared utilities for parsing markdown and creating slides

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, ComrakOptions, markdown_to_html, parse_document};
use toboggan_core::{Content, Slide, SlideKind};

/// Extract text content from an AST node (recursive)
pub fn extract_node_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();

    match &node.data.borrow().value {
        NodeValue::Text(content) => text.push_str(content),
        NodeValue::Code(code) => text.push_str(&code.literal),
        NodeValue::SoftBreak => text.push(' '),
        NodeValue::LineBreak => text.push('\n'),
        _ => {
            for child in node.children() {
                text.push_str(&extract_node_text(child));
            }
        }
    }

    text
}

/// Convert AST node to `CommonMark` markdown
pub fn node_to_commonmark<'a>(node: &'a AstNode<'a>) -> String {
    match &node.data.borrow().value {
        NodeValue::Heading(heading) => {
            let prefix = "#".repeat(heading.level as usize);
            let text = extract_node_text(node);
            format!("{} {}\n\n", prefix, text.trim())
        }
        NodeValue::Paragraph => {
            let text = extract_node_text(node);
            format!("{}\n\n", text.trim())
        }
        NodeValue::BlockQuote => {
            let text = extract_node_text(node);
            text.lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n\n"
        }
        _ => {
            let text = extract_node_text(node);
            if text.trim().is_empty() {
                String::new()
            } else {
                format!("{}\n\n", text.trim())
            }
        }
    }
}

/// Convert markdown to HTML content using comrak
pub fn markdown_to_html_content(markdown: &str) -> Content {
    if markdown.trim().is_empty() {
        return Content::Empty;
    }

    let options = ComrakOptions::default();
    let html = markdown_to_html(markdown, &options);

    Content::Html {
        raw: html.trim().to_string(),
        alt: if markdown.is_empty() {
            None
        } else {
            Some(markdown.to_string())
        },
    }
}

/// Parse slide content from markdown
pub fn parse_slide_from_markdown(content: &str, default_kind: SlideKind) -> Slide {
    let arena = Arena::new();
    let options = ComrakOptions::default();
    let root = parse_document(&arena, content, &options);

    let mut parser = SlideParser::new();
    parser.parse_nodes(root);
    parser.build(default_kind)
}

struct SlideParser {
    title: Option<String>,
    body_content: String,
    notes_content: String,
    in_notes: bool,
    found_title: bool,
}

impl SlideParser {
    fn new() -> Self {
        Self {
            title: None,
            body_content: String::new(),
            notes_content: String::new(),
            in_notes: false,
            found_title: false,
        }
    }

    fn parse_nodes<'a>(&mut self, root: &'a AstNode<'a>) {
        for node in root.children() {
            match &node.data.borrow().value {
                NodeValue::Heading(heading) => self.handle_heading(node, heading.level),
                NodeValue::BlockQuote => self.handle_blockquote(node),
                _ => self.handle_other(node),
            }
        }
    }

    fn handle_heading<'a>(&mut self, node: &'a AstNode<'a>, level: u8) {
        if !self.found_title && level <= 3 {
            self.title = Some(extract_node_text(node));
            self.found_title = true;
        } else if level >= 4 {
            let heading_text = extract_node_text(node);
            if heading_text.to_lowercase().contains("note") {
                self.in_notes = true;
                return;
            }
        }

        if !self.in_notes {
            self.body_content.push_str(&node_to_commonmark(node));
        }
    }

    fn handle_blockquote<'a>(&mut self, node: &'a AstNode<'a>) {
        self.notes_content.push_str(&extract_blockquote_text(node));
    }

    fn handle_other<'a>(&mut self, node: &'a AstNode<'a>) {
        let content = node_to_commonmark(node);
        if self.in_notes {
            self.notes_content.push_str(&content);
        } else {
            self.body_content.push_str(&content);
        }
    }

    fn build(self, default_kind: SlideKind) -> Slide {
        let slide_title = self.title.map_or(Content::text(""), Content::text);
        let slide_body = if self.body_content.trim().is_empty() {
            Content::Empty
        } else {
            markdown_to_html_content(&self.body_content)
        };
        let slide_notes = if self.notes_content.trim().is_empty() {
            None
        } else {
            Some(Content::text(self.notes_content.trim()))
        };

        let slide = match default_kind {
            SlideKind::Cover => Slide::cover(slide_title),
            SlideKind::Part => Slide::part(slide_title),
            SlideKind::Standard => Slide::new(slide_title),
        };

        let slide = slide.with_body(slide_body);
        if let Some(notes) = slide_notes {
            slide.with_notes(notes)
        } else {
            slide
        }
    }
}

fn extract_blockquote_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();

    if let NodeValue::BlockQuote = &node.data.borrow().value {
        for child in node.children() {
            text.push_str(&extract_node_text(child));
            text.push('\n');
        }
    }

    text.trim().to_string()
}

/// Check if file should be processed as a slide
pub fn is_slide_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|extension| extension.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            "md" | "markdown" | "html" | "htm"
        )
    } else {
        false
    }
}

/// Create a slide from a file
pub fn create_slide_from_file(file_path: &Path) -> Result<Slide> {
    let filename = file_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("untitled");

    let slide_kind = match filename {
        "_cover" => SlideKind::Cover,
        "_part" => SlideKind::Part,
        _ => SlideKind::Standard,
    };

    let content = fs::read_to_string(file_path)
        .with_context(|| format!("reading {}", file_path.display()))?;

    let slide = if matches!(
        file_path.extension().and_then(|ext| ext.to_str()),
        Some("html" | "htm")
    ) {
        create_html_slide(&content, slide_kind, filename)
    } else {
        parse_slide_from_markdown(&content, slide_kind)
    };

    Ok(slide)
}

fn create_html_slide(content: &str, slide_kind: SlideKind, filename: &str) -> Slide {
    let html_content = Content::html(content.trim());

    let slide = match slide_kind {
        SlideKind::Cover => Slide::cover(filename),
        SlideKind::Part => Slide::part(filename),
        SlideKind::Standard => Slide::new(filename),
    };

    slide.with_body(html_content)
}
