use std::collections::HashMap;

use toboggan_core::{Slide, SlideId, State};

use crate::config::Config;

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Debug)]
pub struct AppState {
    pub connection_status: ConnectionStatus,
    pub current_slide: Option<SlideId>,
    pub slides: HashMap<SlideId, Slide>,
    pub presentation_state: Option<State>,
    pub show_help: bool,
    pub error_message: Option<String>,
    pub terminal_size: (u16, u16),
    pub config: Config,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            connection_status: ConnectionStatus::Disconnected,
            current_slide: None,
            slides: HashMap::new(),
            presentation_state: None,
            show_help: false,
            error_message: None,
            terminal_size: (80, 24),
            config,
        }
    }

    pub fn update_presentation_state(&mut self, state: State) {
        self.current_slide = Some(state.current());
        self.presentation_state = Some(state);
    }
}
