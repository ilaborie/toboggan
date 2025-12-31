use iced::widget::markdown;
use toboggan_client::ConnectionStatus;
use toboggan_core::{Content, Slide, SlideId, State as PresentationState, Talk};

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
    /// Step counts per slide (from server's `TalkResponse`).
    pub step_counts: Vec<usize>,
    pub cached_markdown: Vec<CachedMarkdown>,
    pub presentation_state: Option<PresentationState>,
    pub current_slide: Option<SlideId>,
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
            step_counts: Vec::new(),
            cached_markdown: Vec::new(),
            presentation_state: None,
            current_slide: None,
            show_help: false,
            show_sidebar: true,
            fullscreen: false,
            error_message: None,
        }
    }
}

impl AppState {
    pub fn current_slide(&self) -> Option<&Slide> {
        self.current_slide
            .and_then(|id| self.slides.get(id.index()))
    }

    pub fn current_markdown(&self) -> Option<&CachedMarkdown> {
        self.current_slide
            .and_then(|id| self.cached_markdown.get(id.index()))
    }

    pub fn next_slide(&self) -> Option<&Slide> {
        if let Some(current_id) = self.current_slide {
            let next_idx = current_id.index() + 1;
            self.slides.get(next_idx)
        } else {
            None
        }
    }

    pub fn slide_index(&self) -> Option<(usize, usize)> {
        self.current_slide
            .map(|id| (id.display_number(), self.slides.len()))
    }

    /// Returns `(current_step, step_count)` for the current slide.
    #[must_use]
    pub fn step_info(&self) -> Option<(usize, usize)> {
        let slide_id = self.current_slide?;
        let step_count = self.step_counts.get(slide_id.index()).copied().unwrap_or(0);
        self.presentation_state
            .as_ref()
            .map(|state| state.step_info(step_count))
    }
}
