use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::state::AppState;

#[derive(Debug, Default)]
pub struct SpeakerNotes {}

impl StatefulWidget for &SpeakerNotes {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let Some(slide) = state.current_slide() else {
            super::render_no_content(area, buf, "no slide active", border::THICK);
            return;
        };

        let block = Block::bordered()
            .title(Line::from(" Notes "))
            .border_set(border::THICK);

        let content_text = slide.notes.to_string();
        let content = super::format_content_lines(&content_text);
        Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
