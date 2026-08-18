use std::ops::ControlFlow;

use toboggan_client::ConnectionStatus;
use toboggan_core::{
    ClientRole, Notification, Slide, SlideId, State, TalkResponse, accumulate_goto, goto_command,
};
use toboggan_stats::SlideStats;
use tracing::{debug, info};

use crate::connection_handler::ConnectionHandler;
use crate::effects::{self, EffectKey, Effects, LayoutAreas};
use crate::events::{AppAction, AppEvent};

#[derive(Debug, Clone, Default)]
pub(crate) enum AppDialog {
    Help,
    Log,
    Error(String),
    #[default]
    None,
}

pub struct AppState {
    pub(crate) connection_status: ConnectionStatus,
    pub(crate) current_slide_id: Option<SlideId>,

    pub(crate) talk: TalkResponse,
    pub(crate) slides: Vec<Slide>,

    pub(crate) presentation_state: State,

    /// The role the server granted, once it has said.
    ///
    /// `None` before the handshake answers, which the title bar shows as
    /// nothing rather than guessing. A client that connects across the network
    /// without a token can watch but not navigate, and used to find that out by
    /// pressing a key and getting an error dialog back.
    pub(crate) role: Option<ClientRole>,

    pub(crate) dialog: AppDialog,
    /// The slide number typed so far, waiting on `Enter`.
    pub(crate) goto_target: Option<usize>,
    pub(crate) terminal_size: (u16, u16),

    pub(crate) effects: Effects,
    pub(crate) layout_areas: LayoutAreas,
}

impl AppState {
    #[must_use]
    pub fn new(talk: TalkResponse, slides: Vec<Slide>) -> Self {
        Self {
            connection_status: ConnectionStatus::Closed,
            current_slide_id: None,
            role: None,
            talk,
            slides,
            presentation_state: State::Init,
            dialog: AppDialog::None,
            goto_target: None,
            terminal_size: (80, 24),
            effects: Effects::default(),
            layout_areas: LayoutAreas::default(),
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
    pub(crate) fn handle_event(
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
                let was_connected = self.is_connected();
                if let ConnectionStatus::Error { message } = &status {
                    self.dialog = AppDialog::Error(message.clone());
                }
                self.connection_status = status;
                let is_connected = self.is_connected();
                if was_connected != is_connected {
                    let area = self.layout_areas.title_bar;
                    self.effects.add_unique_effect(
                        EffectKey::ConnectionPulse,
                        effects::connection_pulse_effect(area, is_connected),
                    );
                }
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
        match action {
            AppAction::Digit(digit) => {
                self.push_goto_digit(digit);
                return ControlFlow::Continue(());
            }
            AppAction::GotoTyped => {
                if let Some(number) = self.goto_target.take() {
                    connection_handler.send_command(&goto_command(number));
                }
                return ControlFlow::Continue(());
            }
            // Anything else abandons a half-typed number rather than leaving it
            // to land on some later `Enter`.
            _ => self.goto_target = None,
        }

        let was_no_dialog = matches!(self.dialog, AppDialog::None);
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
        // Trigger dialog open effect
        if was_no_dialog && !matches!(self.dialog, AppDialog::None) {
            // Dialog area will be computed during render; use content area as approximation
            let area = self.layout_areas.content;
            self.effects
                .add_unique_effect(EffectKey::DialogOverlay, effects::dialog_open_effect(area));
        }
        ControlFlow::Continue(())
    }

    /// Appends a digit to the slide number being typed.
    ///
    /// The arithmetic is [`accumulate_goto`]'s, shared with the web client —
    /// which had its own copy, with the same leading-zero defect.
    fn push_goto_digit(&mut self, digit: u8) {
        if let Some(typed) = accumulate_goto(self.goto_target, digit) {
            self.goto_target = Some(typed);
        }
    }

    fn handle_notification(&mut self, notification: Notification) {
        match notification {
            Notification::State { state } | Notification::TalkChange { state } => {
                self.apply_state_change(state);
            }
            Notification::Blink => {
                self.effects
                    .add_unique_effect(EffectKey::Blink, effects::blink_effect());
            }
            Notification::Registered { role, .. } => {
                self.role = Some(role);
            }
            Notification::Pong
            | Notification::ClientConnected { .. }
            | Notification::ClientDisconnected { .. } => {}
            Notification::Error { message } => {
                self.dialog = AppDialog::Error(message);
            }
        }
    }

    #[cfg(test)]
    fn type_goto(&mut self, digits: &str) {
        for digit in digits.chars().filter_map(|ch| ch.to_digit(10)) {
            self.push_goto_digit(u8::try_from(digit).unwrap_or(0));
        }
    }

    fn apply_state_change(&mut self, new_state: State) {
        let old_slide_id = self.current_slide_id;
        let was_init = matches!(self.presentation_state, State::Init);
        let was_done = matches!(self.presentation_state, State::Done { .. });

        self.current_slide_id = new_state.current();
        self.presentation_state = new_state;

        let slide_area = self.layout_areas.current_slide;

        // Init -> Running: whole UI materializes
        if was_init && matches!(self.presentation_state, State::Running { .. }) {
            self.effects.add_unique_effect(
                EffectKey::StateTransition,
                effects::init_to_running_effect(self.layout_areas.content),
            );
            return;
        }

        // Running -> Done: celebratory glow
        if !was_done && matches!(self.presentation_state, State::Done { .. }) {
            self.effects.add_unique_effect(
                EffectKey::StateTransition,
                effects::running_to_done_effect(),
            );
        }

        // Slide change or step change effects
        if old_slide_id != self.current_slide_id {
            self.effects.add_unique_effect(
                EffectKey::SlideTransition,
                effects::slide_transition_effect(slide_area),
            );
        } else if old_slide_id.is_some() {
            self.effects.add_unique_effect(
                EffectKey::StepReveal,
                effects::step_reveal_effect(slide_area),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_state() -> AppState {
        AppState::new(TalkResponse::default(), vec![])
    }

    /// Nine slides was the ceiling: one keystroke, one jump, and no way to
    /// reach the tenth. Digits accumulate now.
    #[test]
    fn digits_accumulate_into_one_slide_number() {
        let mut state = empty_state();
        state.type_goto("127");
        assert_eq!(state.goto_target, Some(127));
    }

    /// A leaned-on key would otherwise overflow the running multiplication.
    #[test]
    fn digits_past_the_cap_are_dropped_rather_than_wrapping() {
        let mut state = empty_state();
        state.type_goto("99999999");
        assert_eq!(state.goto_target, Some(9_999));
    }
}
