use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Paragraph};
use toboggan_core::ClientRole;

use crate::events::AppAction;
use crate::state::AppState;
use crate::ui::styles;
use crate::ui::styles::colors;
use crate::ui::widgets::line_from_actions;

#[derive(Debug, Default)]
pub struct TitleBar {}

impl StatefulWidget for &TitleBar {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let color = if state.is_connected() {
            colors::GREEN
        } else {
            colors::RED
        };

        // A slide number being typed takes the bar over while it lasts: the
        // digits have to show up somewhere, or the presenter cannot tell
        // whether the TUI is listening or where they are about to land.
        let title = match state.goto_target {
            Some(number) => Line::from(vec![
                Span::raw(" "),
                Span::styled(format!("→ slide {number} ⏎"), styles::action::KEY),
                Span::raw(" "),
            ]),
            // An audience client is said so here rather than left to be
            // discovered by pressing a key and being refused.
            None if state.role == Some(ClientRole::Audience) => Line::from(vec![
                Span::raw(" "),
                Span::raw(state.connection_status.to_string()),
                Span::raw(" · "),
                Span::styled("watching", Style::default().fg(colors::YELLOW)),
                Span::raw(" "),
            ]),
            None => Line::from(vec![
                Span::raw(" "),
                Span::raw(state.connection_status.to_string()),
                Span::raw(" "),
            ]),
        };
        let actions = global_actions(state);
        let bottom = line_from_actions(&actions);

        let block = Block::bordered()
            .border_style(Style::default().fg(color))
            .title(title.centered())
            .title_bottom(bottom.centered())
            .border_set(border::DOUBLE);

        let title = state.talk.title.clone();
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

fn global_actions(_state: &AppState) -> Vec<AppAction> {
    vec![
        AppAction::Blink,
        AppAction::ShowLog,
        AppAction::Quit,
        AppAction::Help,
    ]
}
