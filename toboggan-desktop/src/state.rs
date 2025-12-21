use iced::Theme;
use iced::widget::markdown;
use toboggan_client::ConnectionStatus;
use toboggan_core::{Content, Slide, State as PresentationState, Talk};

/// Cached markdown content for a slide
#[derive(Debug, Clone, Default)]
pub struct CachedMarkdown {
    pub body_items: Vec<markdown::Item>,
    pub notes_items: Vec<markdown::Item>,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub connection_status: ConnectionStatus,
    pub talk: Option<Talk>,
    pub slides: Vec<Slide>,
    pub cached_markdown: Vec<CachedMarkdown>,
    pub presentation_state: Option<PresentationState>,
    pub current_slide_index: Option<usize>,
    pub show_help: bool,
    pub show_sidebar: bool,
    pub fullscreen: bool,
    pub error_message: Option<String>,
}

/// Parse Content to markdown text
fn content_to_markdown_text(content: &Content) -> String {
    match content {
        Content::Empty => String::new(),
        Content::Text { text } => text.clone(),
        Content::Html { raw, alt, .. } => alt.as_ref().unwrap_or(raw).clone(),
        Content::Grid { cells, .. } => cells
            .iter()
            .map(content_to_markdown_text)
            .collect::<Vec<_>>()
            .join("\n\n"),
    }
}

/// Parse slides into cached markdown items
pub fn parse_slides_markdown(slides: &[Slide]) -> Vec<CachedMarkdown> {
    slides
        .iter()
        .map(|slide| {
            let body_text = content_to_markdown_text(&slide.body);
            let notes_text = content_to_markdown_text(&slide.notes);

            CachedMarkdown {
                body_items: markdown::parse(&body_text).collect(),
                notes_items: markdown::parse(&notes_text).collect(),
            }
        })
        .collect()
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connection_status: ConnectionStatus::Closed,
            talk: None,
            slides: Vec::new(),
            cached_markdown: Vec::new(),
            presentation_state: None,
            current_slide_index: None,
            show_help: false,
            show_sidebar: true,
            fullscreen: false,
            error_message: None,
        }
    }
}

impl AppState {
    pub fn current_slide(&self) -> Option<&Slide> {
        self.current_slide_index
            .and_then(|idx| self.slides.get(idx))
    }

    pub fn current_markdown(&self) -> Option<&CachedMarkdown> {
        self.current_slide_index
            .and_then(|idx| self.cached_markdown.get(idx))
    }

    pub fn next_slide(&self) -> Option<&Slide> {
        if let Some(current_idx) = self.current_slide_index {
            let next_idx = current_idx + 1;
            self.slides.get(next_idx)
        } else {
            None
        }
    }

    pub fn slide_index(&self) -> Option<(usize, usize)> {
        self.current_slide_index
            .map(|current_idx| (current_idx + 1, self.slides.len()))
    }

    /// Returns `(current_step, step_count)` for the current slide
    #[must_use]
    pub fn step_info(&self) -> Option<(usize, usize)> {
        let slide = self.current_slide()?;
        let current_step = self
            .presentation_state
            .as_ref()
            .map_or(0, PresentationState::current_step);
        Some((current_step, slide.step_count))
    }

    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn theme(&self) -> Theme {
        // Takes &self for future customization (e.g., user preferences)
        Theme::Dark
    }
}
