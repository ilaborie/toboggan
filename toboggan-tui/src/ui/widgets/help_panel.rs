use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Paragraph};

use crate::events::{ActionDetails, AppAction};
use crate::ui::styles;

#[derive(Debug, Default)]
pub struct HelpPanel {}

impl Widget for &HelpPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title(" Help ")
            .border_set(border::ROUNDED);

        let mut content = vec![];
        content.extend(build_lines(
            "Navigation",
            &[
                AppAction::First,
                AppAction::Previous,
                AppAction::Goto(1),
                AppAction::Next,
                AppAction::Last,
            ],
        ));

        content.extend(build_lines(
            "Presentation",
            &[AppAction::Pause, AppAction::Resume, AppAction::Blink],
        ));

        content.extend(build_lines(
            "Application",
            &[
                AppAction::Close,
                AppAction::ShowLog,
                AppAction::Quit,
                AppAction::Help,
            ],
        ));

        Paragraph::new(content).block(block).render(area, buf);
    }
}

fn build_lines<'a>(title: &'a str, actions: &'a [AppAction]) -> Vec<Line<'a>> {
    let mut lines = vec![];
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" ❖ {title}"),
        styles::action::TITLE,
    )));
    for action in actions {
        let ActionDetails { keys, description } = action.details();
        let mut keys_len = 0;
        let mut spans = vec![];
        spans.push(Span::raw(" "));
        for key in keys {
            spans.push(Span::raw(" "));
            let key = format!("[{key}]");
            keys_len += key.len() + 1;
            spans.push(Span::styled(key, styles::action::KEY));
        }

        spans.push(Span::raw(" ".repeat(24 - keys_len)));
        spans.push(Span::raw(" · "));
        spans.push(Span::styled(description, styles::action::DESCRIPTION));

        lines.push(Line::from(spans));
    }

    lines
}
