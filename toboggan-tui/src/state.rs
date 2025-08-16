use std::collections::HashMap;
use std::ops::ControlFlow;

use toboggan_client::ConnectionStatus;
use toboggan_core::{Notification, Slide, SlideId, State, TalkResponse};
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
    pub(crate) current_slide: Option<SlideId>,

    pub(crate) talk: TalkResponse,
    pub(crate) slides: HashMap<SlideId, Slide>,
    pub(crate) ids: Vec<SlideId>,

    pub(crate) presentation_state: State,

    pub(crate) dialog: AppDialog,
    pub(crate) terminal_size: (u16, u16),
}

impl AppState {
    #[must_use]
    pub fn new(talk: TalkResponse, slides: Vec<Slide>) -> Self {
        let ids = slides.iter().map(|slide| slide.id).collect();
        let slides = slides.into_iter().map(|slide| (slide.id, slide)).collect();

        Self {
            connection_status: ConnectionStatus::Closed,
            current_slide: None,
            talk,
            slides,
            ids,
            presentation_state: State::Init,
            dialog: AppDialog::None,
            terminal_size: (80, 24),
        }
    }

    pub(crate) fn is_connected(&self) -> bool {
        matches!(self.connection_status, ConnectionStatus::Connected)
    }

    pub(crate) fn current(&self) -> u8 {
        self.current_slide
            .and_then(|id| id.to_string().parse::<u8>().ok())
            .unwrap_or_default()
    }

    pub(crate) fn count(&self) -> usize {
        self.talk.titles.len()
    }

    pub(crate) fn is_first_slide(&self) -> bool {
        self.current_slide == self.ids.first().copied()
    }

    pub(crate) fn is_last_slide(&self) -> bool {
        self.current_slide == self.ids.last().copied()
    }

    pub(crate) fn current_slide(&self) -> Option<&Slide> {
        let current_id = self.current_slide?;
        self.slides.get(&current_id)
    }

    pub(crate) fn next_slide(&self) -> Option<&Slide> {
        let current_id = self.current_slide?;
        let pos = self.ids.iter().position(|it| current_id == *it)?;
        let next = self.ids.get(pos + 1)?;
        self.slides.get(next)
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
            Notification::State { state, .. } => {
                self.current_slide = state.current();
                self.presentation_state = state;
            }
            Notification::Pong { .. } | Notification::Blink => {
                // TODO
            }
            Notification::Error { message, .. } => {
                self.dialog = AppDialog::Error(message);
            }
        }
    }
}
