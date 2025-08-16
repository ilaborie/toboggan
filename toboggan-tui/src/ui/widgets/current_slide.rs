use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::events::AppAction;
use crate::state::AppState;
use crate::ui::styles;
use crate::ui::widgets::line_from_actions;

#[derive(Debug, Default)]
pub struct CurrentSlide {}

impl StatefulWidget for &CurrentSlide {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let Some(slide) = state.current_slide() else {
            super::render_no_content(area, buf, "no slide active", border::DOUBLE);
            return;
        };

        let title = Line::from(vec![
            Span::raw(" "),
            Span::raw(slide.title.to_string()),
            Span::raw(" "),
        ]);

        let actions = slide_actions(state);
        let bottom = line_from_actions(&actions);

        let kind = styles::slide::get_slide_kind_span(slide.kind);

        let block = Block::bordered()
            .title(Line::from(kind).right_aligned())
            .title(title.bold())
            .title_bottom(bottom.centered())
            .border_set(border::DOUBLE);

        let content_text = slide.body.to_string();
        let content = super::format_content_lines(&content_text);
        Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}

fn slide_actions(state: &AppState) -> Vec<AppAction> {
    let mut actions = vec![];

    if !state.is_first_slide() {
        actions.extend([AppAction::First, AppAction::Previous]);
    }

    if !state.is_last_slide() {
        actions.extend([AppAction::Next, AppAction::Last]);
    }

    actions
}
