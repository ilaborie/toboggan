//! Flat markdown file parser implementation

use anyhow::Result;
use comrak::nodes::NodeValue;
use comrak::{Arena, ComrakOptions, parse_document};
use toboggan_core::{Content, Date, Slide, SlideKind, Talk};

use super::{Parser, utils};

/// Parser for flat markdown files with slide separators
pub struct FlatFileParser {
    content: String,
}

impl FlatFileParser {
    /// Create a new flat file parser from content
    #[must_use]
    pub fn new(content: String) -> Self {
        Self { content }
    }
}

impl Parser for FlatFileParser {
    fn parse(&self, title_override: Option<Content>, date_override: Option<Date>) -> Result<Talk> {
        let arena = Arena::new();
        let options = ComrakOptions::default();
        let root = parse_document(&arena, &self.content, &options);

        let mut builder = TalkBuilder::new();
        builder.extract_title(root);
        builder.process_slides(root);
        Ok(builder.build(title_override, date_override))
    }
}

struct TalkBuilder {
    title_content: Option<String>,
    slides: Vec<Slide>,
    current_slide_content: String,
    found_title: bool,
    in_slide: bool,
}

impl TalkBuilder {
    fn new() -> Self {
        Self {
            title_content: None,
            slides: Vec::new(),
            current_slide_content: String::new(),
            found_title: false,
            in_slide: false,
        }
    }

    fn extract_title<'a>(&mut self, root: &'a comrak::nodes::AstNode<'a>) {
        for node in root.children() {
            if let NodeValue::Heading(ref heading) = node.data.borrow().value
                && heading.level == 1
                && !self.found_title
            {
                self.title_content = Some(utils::extract_node_text(node));
                self.found_title = true;
                break;
            }
        }
    }

    fn process_slides<'a>(&mut self, root: &'a comrak::nodes::AstNode<'a>) {
        for node in root.children() {
            match &node.data.borrow().value {
                NodeValue::Heading(heading) => self.handle_heading(node, heading.level),
                NodeValue::ThematicBreak => self.handle_thematic_break(),
                _ => self.handle_content(node),
            }
        }

        // Handle last slide
        self.finalize_current_slide(SlideKind::Standard);
    }

    fn handle_heading<'a>(&mut self, node: &'a comrak::nodes::AstNode<'a>, level: u8) {
        if level == 1 && self.found_title {
            return; // Skip the title heading
        }

        if level >= 2 {
            let slide_kind = if level == 2 {
                SlideKind::Part
            } else {
                SlideKind::Standard
            };
            self.finalize_current_slide(slide_kind);
            self.in_slide = true;
        }

        self.current_slide_content
            .push_str(&utils::node_to_commonmark(node));
    }

    fn handle_thematic_break(&mut self) {
        self.finalize_current_slide(SlideKind::Standard);
        self.in_slide = false;
    }

    fn handle_content<'a>(&mut self, node: &'a comrak::nodes::AstNode<'a>) {
        if self.in_slide {
            self.current_slide_content
                .push_str(&utils::node_to_commonmark(node));
        }
    }

    fn finalize_current_slide(&mut self, slide_kind: SlideKind) {
        if self.in_slide && !self.current_slide_content.trim().is_empty() {
            let slide = utils::parse_slide_from_markdown(&self.current_slide_content, slide_kind);
            self.slides.push(slide);
            self.current_slide_content.clear();
        }
    }

    fn build(mut self, title_override: Option<Content>, date_override: Option<Date>) -> Talk {
        // Convert first slide to cover if needed
        if let Some(first_slide) = self.slides.get_mut(0)
            && matches!(first_slide.kind, SlideKind::Standard)
        {
            *first_slide = Slide::cover(first_slide.title.clone())
                .with_body(first_slide.body.clone())
                .with_notes(first_slide.notes.clone());
        }

        let title = title_override
            .or_else(|| self.title_content.map(Content::text))
            .unwrap_or_else(|| Content::text("Untitled"));

        let mut talk = Talk::new(title);
        if let Some(date) = date_override {
            talk = talk.with_date(date);
        }

        for slide in self.slides {
            talk = talk.add_slide(slide);
        }

        talk
    }
}
