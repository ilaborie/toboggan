//! Editable talk state

use toboggan_core::{Content, Date, Slide, SlideKind, Style, Talk};

/// Editable talk state
#[derive(Debug, Clone)]
pub struct TalkState {
    pub title: String,
    pub date: Date,
    pub footer: Option<String>,
    pub head: Option<String>,
    pub cover: Option<SlideState>,
    pub parts: Vec<PartState>,
    /// Slides not in any part (at root level)
    pub loose_slides: Vec<SlideState>,
}

/// Editable part (section) state
#[derive(Debug, Clone)]
pub struct PartState {
    pub title: String,
    pub body: String,
    pub notes: String,
    pub classes: Vec<String>,
    pub inline_style: Option<String>,
    pub slides: Vec<SlideState>,
}

/// Editable slide state
#[derive(Debug, Clone)]
pub struct SlideState {
    pub kind: SlideKind,
    pub title: String,
    pub body: String,
    pub notes: String,
    pub classes: Vec<String>,
    pub inline_style: Option<String>,
    pub skip: bool,
}

impl TalkState {
    /// Create a new empty talk
    #[must_use]
    pub fn new(title: String, date: Date) -> Self {
        Self {
            title,
            date,
            footer: None,
            head: None,
            cover: Some(SlideState::new_cover()),
            parts: Vec::new(),
            loose_slides: Vec::new(),
        }
    }

    /// Convert from toboggan-core Talk
    #[must_use]
    pub fn from_core(talk: Talk) -> Self {
        let mut cover = None;
        let mut parts = Vec::new();
        let mut current_part: Option<PartState> = None;
        let mut loose_slides = Vec::new();

        for slide in talk.slides {
            match slide.kind {
                SlideKind::Cover => {
                    cover = Some(SlideState::from_core_slide(slide));
                }
                SlideKind::Part => {
                    // Save previous part if exists
                    if let Some(part) = current_part.take() {
                        parts.push(part);
                    }
                    // Start new part
                    current_part = Some(PartState::from_core_slide(slide));
                }
                SlideKind::Standard => {
                    if let Some(ref mut part) = current_part {
                        part.slides.push(SlideState::from_core_slide(slide));
                    } else {
                        loose_slides.push(SlideState::from_core_slide(slide));
                    }
                }
            }
        }

        // Don't forget last part
        if let Some(part) = current_part {
            parts.push(part);
        }

        Self {
            title: talk.title,
            date: talk.date,
            footer: talk.footer,
            head: talk.head,
            cover,
            parts,
            loose_slides,
        }
    }

    /// Convert to toboggan-core Talk
    #[must_use]
    pub fn to_core(&self) -> Talk {
        let mut slides = Vec::new();

        // Add cover
        if let Some(cover) = &self.cover {
            slides.push(cover.to_core_slide());
        }

        // Add loose slides first
        for slide in &self.loose_slides {
            slides.push(slide.to_core_slide());
        }

        // Add parts with their slides
        for part in &self.parts {
            slides.push(part.to_core_slide());
            for slide in &part.slides {
                slides.push(slide.to_core_slide());
            }
        }

        Talk {
            title: self.title.clone(),
            date: self.date,
            footer: self.footer.clone(),
            head: self.head.clone(),
            slides,
        }
    }

    /// Total number of slides (including cover and parts)
    #[must_use]
    pub fn total_slides(&self) -> usize {
        let cover = usize::from(self.cover.is_some());
        let parts = self.parts.len();
        let part_slides: usize = self.parts.iter().map(|part| part.slides.len()).sum();
        let loose = self.loose_slides.len();
        cover + parts + part_slides + loose
    }

    /// Calculate total word count for the entire talk
    pub fn word_count(&self) -> usize {
        let cover_words = self.cover.as_ref().map_or(0, SlideState::word_count);

        let loose_words: usize = self.loose_slides.iter().map(SlideState::word_count).sum();

        let part_words: usize = self.parts.iter().map(PartState::word_count).sum();

        cover_words + loose_words + part_words
    }

    /// Add a new part
    pub fn add_part(&mut self) -> usize {
        let index = self.parts.len();
        self.parts
            .push(PartState::new(format!("Part {}", index + 1)));
        index
    }

    /// Add a new slide to a part
    pub fn add_slide_to_part(&mut self, part_index: usize) -> Option<usize> {
        let part = self.parts.get_mut(part_index)?;
        let index = part.slides.len();
        part.slides.push(SlideState::new());
        Some(index)
    }

    /// Add a loose slide
    pub fn add_loose_slide(&mut self) -> usize {
        let index = self.loose_slides.len();
        self.loose_slides.push(SlideState::new());
        index
    }

    /// Delete a part by index
    pub fn delete_part(&mut self, index: usize) -> Option<PartState> {
        (index < self.parts.len()).then(|| self.parts.remove(index))
    }

    /// Delete a slide from a part
    pub fn delete_slide_from_part(
        &mut self,
        part_index: usize,
        slide_index: usize,
    ) -> Option<SlideState> {
        let part = self.parts.get_mut(part_index)?;
        (slide_index < part.slides.len()).then(|| part.slides.remove(slide_index))
    }

    /// Delete a loose slide
    pub fn delete_loose_slide(&mut self, index: usize) -> Option<SlideState> {
        (index < self.loose_slides.len()).then(|| self.loose_slides.remove(index))
    }
}

impl PartState {
    #[must_use]
    pub fn new(title: String) -> Self {
        Self {
            title,
            body: String::new(),
            notes: String::new(),
            classes: Vec::new(),
            inline_style: None,
            slides: Vec::new(),
        }
    }

    #[must_use]
    pub fn from_core_slide(slide: Slide) -> Self {
        Self {
            title: content_to_string(&slide.title),
            body: content_to_string(&slide.body),
            notes: content_to_string(&slide.notes),
            classes: slide.style.classes,
            inline_style: slide.style.style,
            slides: Vec::new(),
        }
    }

    #[must_use]
    pub fn to_core_slide(&self) -> Slide {
        Slide {
            kind: SlideKind::Part,
            title: string_to_content(&self.title),
            body: string_to_content(&self.body),
            notes: string_to_content(&self.notes),
            style: Style {
                classes: self.classes.clone(),
                style: self.inline_style.clone(),
            },
        }
    }

    /// Calculate word count for this part (including all slides)
    pub fn word_count(&self) -> usize {
        let part_words = count_words(&self.body) + count_words(&self.notes);
        let slide_words: usize = self.slides.iter().map(SlideState::word_count).sum();
        part_words + slide_words
    }
}

impl Default for SlideState {
    fn default() -> Self {
        Self::new()
    }
}

impl SlideState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            kind: SlideKind::Standard,
            title: String::new(),
            body: String::new(),
            notes: String::new(),
            classes: Vec::new(),
            inline_style: None,
            skip: false,
        }
    }

    #[must_use]
    pub fn new_cover() -> Self {
        Self {
            kind: SlideKind::Cover,
            ..Self::new()
        }
    }

    #[must_use]
    pub fn from_core_slide(slide: Slide) -> Self {
        Self {
            kind: slide.kind,
            title: content_to_string(&slide.title),
            body: content_to_string(&slide.body),
            notes: content_to_string(&slide.notes),
            classes: slide.style.classes,
            inline_style: slide.style.style,
            skip: false, // Note: skip is in frontmatter, not core type
        }
    }

    #[must_use]
    pub fn to_core_slide(&self) -> Slide {
        Slide {
            kind: self.kind,
            title: string_to_content(&self.title),
            body: string_to_content(&self.body),
            notes: string_to_content(&self.notes),
            style: Style {
                classes: self.classes.clone(),
                style: self.inline_style.clone(),
            },
        }
    }

    /// Calculate word count for this slide
    #[must_use]
    pub fn word_count(&self) -> usize {
        count_words(&self.body) + count_words(&self.notes)
    }

    /// Title preview length limit for sidebar display
    const TITLE_PREVIEW_LIMIT: usize = 30;

    /// Get the display title (returns "Untitled" if empty)
    #[must_use]
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            "Untitled"
        } else {
            &self.title
        }
    }

    /// Get a preview of the title for the sidebar (truncated if too long)
    #[must_use]
    pub fn title_preview(&self) -> &str {
        let title = self.display_title();
        if title.len() > Self::TITLE_PREVIEW_LIMIT {
            &title[..Self::TITLE_PREVIEW_LIMIT]
        } else {
            title
        }
    }
}

/// Count words in text
fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Convert Content to String
fn content_to_string(content: &Content) -> String {
    match content {
        Content::Empty => String::new(),
        Content::Text { text } => text.clone(),
        Content::Html { raw, .. } => raw.clone(),
    }
}

/// Convert String to Content
fn string_to_content(text: &str) -> Content {
    if text.is_empty() {
        Content::Empty
    } else {
        Content::text(text)
    }
}
