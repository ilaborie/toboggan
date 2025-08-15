use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Paragraph};
use toboggan_core::State;

use crate::events::AppAction;
use crate::state::AppState;
use crate::ui::styles::colors;
use crate::ui::widgets::line_from_actions;

#[derive(Debug, Default)]
pub struct TitleBar {}

impl StatefulWidget for TitleBar {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let color = if state.is_connected() {
            colors::GREEN
        } else {
            colors::RED
        };

        let title = Line::from(vec![
            Span::raw(" "),
            Span::raw(state.connection_status.to_string()),
            Span::raw(" "),
        ]);
        let actions = global_actions(state);
        let bottom = line_from_actions(&actions);

        let block = Block::bordered()
            .border_style(Style::default().fg(color))
            .title(title.centered())
            .title_bottom(bottom.centered())
            .border_set(border::DOUBLE);

        let title = state.talk.title.to_string();
        let date = state.talk.date.to_string();
        let content = Line::from(vec![
            Span::raw(title).bold(),
            Span::raw(" - "),
            Span::raw(date),
        ]);

        Paragraph::new(content)
            .centered()
            .block(block)
            .render(area, buf);
    }
}

fn global_actions(state: &AppState) -> Vec<AppAction> {
    let mut actions = vec![AppAction::Blink];
    match state.presentation_state {
        State::Paused { .. } => actions.push(AppAction::Resume),
        State::Running { .. } => actions.push(AppAction::Pause),
        State::Init | State::Done { .. } => {}
    }

    actions.push(AppAction::ShowLog);
    actions.push(AppAction::Quit);
    actions.push(AppAction::Help);

    actions
}
