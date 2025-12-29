use std::ops::ControlFlow;

use toboggan_client::ConnectionStatus;
use toboggan_core::{Notification, Slide, SlideId, State, TalkResponse};
use toboggan_stats::SlideStats;
use tracing::{debug, info};

use crate::connection_handler::ConnectionHandler;
use crate::events::{AppAction, AppEvent};

#[derive(Debug, Clone, Default)]
pub enum AppDialog {
    Help,
    Log,
    Error(String),
    #[default]
    None,
}

pub struct AppState {
    // pub(crate) config: Config,
    pub(crate) connection_status: ConnectionStatus,
    pub(crate) current_slide_id: Option<SlideId>,

    pub(crate) talk: TalkResponse,
    pub(crate) slides: Vec<Slide>,

    pub(crate) presentation_state: State,

    pub(crate) dialog: AppDialog,
    pub(crate) terminal_size: (u16, u16),
}

impl AppState {
    #[must_use]
    pub fn new(talk: TalkResponse, slides: Vec<Slide>) -> Self {
        Self {
            connection_status: ConnectionStatus::Closed,
            current_slide_id: None,
            talk,
            slides,
            presentation_state: State::Init,
            dialog: AppDialog::None,
            terminal_size: (80, 24),
        }
    }

    pub(crate) fn is_connected(&self) -> bool {
        matches!(self.connection_status, ConnectionStatus::Connected)
    }

    pub(crate) fn current(&self) -> usize {
        self.current_slide_id.map_or(0, SlideId::index)
    }

    pub(crate) fn count(&self) -> usize {
        self.talk.titles.len()
    }

    pub(crate) fn is_first_slide(&self) -> bool {
        self.presentation_state.is_first_slide(self.slides.len())
    }

    pub(crate) fn is_last_slide(&self) -> bool {
        self.presentation_state.is_last_slide(self.slides.len())
    }

    pub(crate) fn current_slide(&self) -> Option<&Slide> {
        let current_id = self.current_slide_id?;
        self.slides.get(current_id.index())
    }

    pub(crate) fn next_slide(&self) -> Option<&Slide> {
        let current_id = self.current_slide_id?;
        self.slides.get(current_id.index() + 1)
    }

    /// Returns `(current_step, step_count)` for the current slide.
    #[must_use]
    pub(crate) fn step_info(&self) -> Option<(usize, usize)> {
        let slide = self.current_slide()?;
        let step_count = SlideStats::from_slide(slide).steps;
        Some(self.presentation_state.step_info(step_count))
    }

    // Event handling methods
    pub fn handle_event(
        &mut self,
        event: AppEvent,
        connection_handler: &ConnectionHandler,
    ) -> ControlFlow<()> {
        // if !matches!(event, AppEvent::Tick) {
        //     debug!("Handling event: {event:?}");
        // }

        match event {
            AppEvent::Key(key) => {
                debug!("Handling key event: {key:?}");
                let action = AppAction::from_key(key);
                if let Some(action) = action {
                    return self.handle_action(action, connection_handler);
                }
            }
            AppEvent::ConnectionStatus(status) => {
                info!("{status}");
                if let ConnectionStatus::Error { message } = &status {
                    self.dialog = AppDialog::Error(message.clone());
                }
                self.connection_status = status;
            }
            AppEvent::NotificationReceived(notification) => {
                self.handle_notification(notification);
            }
            AppEvent::TalkAndSlidesRefetched(talk, slides) => {
                info!("📝 Updating talk and slides from refetch");
                self.talk = *talk;
                self.slides = slides;
            }
            AppEvent::Error(error) => {
                self.dialog = AppDialog::Error(error);
            }
            AppEvent::Tick => {}
        }

        ControlFlow::Continue(())
    }

    fn handle_action(
        &mut self,
        action: AppAction,
        connection_handler: &ConnectionHandler,
    ) -> ControlFlow<()> {
        self.dialog = match action {
            AppAction::Close => AppDialog::None,
            AppAction::Help => AppDialog::Help,
            AppAction::ShowLog => AppDialog::Log,
            AppAction::Quit => {
                return ControlFlow::Break(());
            }
            _ => {
                if let Some(cmd) = action.command() {
                    connection_handler.send_command(&cmd);
                }
                AppDialog::None
            }
        };
        ControlFlow::Continue(())
    }

    fn handle_notification(&mut self, notification: Notification) {
        match notification {
            Notification::State { state } => {
                self.current_slide_id = state.current();
                self.presentation_state = state;
            }
            Notification::TalkChange { state } => {
                // Presentation updated - state already has correct slide position
                self.current_slide_id = state.current();
                self.presentation_state = state;
            }
            Notification::Pong
            | Notification::Blink
            | Notification::Registered { .. }
            | Notification::ClientConnected { .. }
            | Notification::ClientDisconnected { .. } => {
                // Pong: heartbeat response, no UI action needed
                // Blink: visual effect not implemented in TUI
                // Client registration events - no UI action needed in TUI
            }
            Notification::Error { message } => {
                self.dialog = AppDialog::Error(message);
            }
        }
    }
}
