use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::state::AppState;
use crate::ui::styles;

#[derive(Debug, Default)]
pub struct SpeakerNotes {}

impl StatefulWidget for &SpeakerNotes {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let Some(slide) = state.current_slide() else {
            let title = Line::from(Span::styled(
                " <no slide active> ",
                styles::ui::NO_CONTENT_STYLE,
            ));
            let block = Block::bordered().title(title).border_set(border::THICK);
            let content = vec![];
            Paragraph::new(content).block(block).render(area, buf);
            return;
        };

        let block = Block::bordered()
            .title(Line::from(" Notes "))
            .border_set(border::THICK);

        let content_text = slide.notes.to_string();
        let content = content_text.lines().map(Line::from).collect::<Vec<_>>();
        Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
