use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Paragraph};

use crate::events::{ActionDetails, AppAction};
use crate::ui::styles;

/// Character width of the key column, before the `·` separator.
const KEYS_COLUMN: usize = 24;

#[derive(Debug, Default)]
pub struct HelpPanel {}

impl Widget for &HelpPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title(" Help ")
            .border_set(border::ROUNDED);

        let mut content = vec![];
        content.extend(build_lines(
            "Step Navigation",
            &[AppAction::PreviousStep, AppAction::NextStep],
        ));

        content.extend(build_lines(
            "Slide Navigation",
            &[
                AppAction::First,
                AppAction::Previous,
                AppAction::Next,
                AppAction::Last,
                AppAction::Goto(1),
            ],
        ));

        content.extend(build_lines("Presentation", &[AppAction::Blink]));

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
        // Counted in characters, not bytes, and clamped: `↑` is three bytes on
        // its own, and a third key on a row was enough to take the byte count
        // past the column and underflow the subtraction.
        let mut keys_width = 0;
        let mut spans = vec![];
        spans.push(Span::raw(" "));
        for key in keys {
            spans.push(Span::raw(" "));
            let key = format!("[{key}]");
            keys_width += key.chars().count() + 1;
            spans.push(Span::styled(key, styles::action::KEY));
        }

        spans.push(Span::raw(
            " ".repeat(KEYS_COLUMN.saturating_sub(keys_width)),
        ));
        spans.push(Span::raw(" · "));
        spans.push(Span::styled(description, styles::action::DESCRIPTION));

        lines.push(Line::from(spans));
    }

    lines
}
