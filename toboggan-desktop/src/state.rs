use toboggan_client::ConnectionStatus;
use toboggan_core::{Slide, State as PresentationState, Talk};

#[derive(Debug, Clone)]
pub struct AppState {
    pub connection_status: ConnectionStatus,
    pub talk: Option<Talk>,
    pub slides: Vec<Slide>,
    pub presentation_state: Option<PresentationState>,
    pub current_slide_index: Option<usize>,
    pub show_help: bool,
    pub show_sidebar: bool,
    pub fullscreen: bool,
    pub error_message: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connection_status: ConnectionStatus::Closed,
            talk: None,
            slides: Vec::new(),
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
}
