use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Paragraph, Wrap};

use crate::state::AppState;

#[derive(Debug, Default)]
pub struct NextSlidePreview {}

impl StatefulWidget for &NextSlidePreview {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let Some(slide) = state.next_slide() else {
            let content = vec![];
            Paragraph::new(content).render(area, buf);
            return;
        };

        let block = Block::bordered()
            .title(Line::from(" Next "))
            .border_set(border::PLAIN);

        let area = area.inner(Margin::new(1, 1));
        let title = slide.title.to_string();
        let content = Line::from(title);

        Paragraph::new(content)
            .block(block)
            .centered()
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
