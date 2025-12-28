use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, List, ListItem, ListState};
use toboggan_core::SlideId;

use crate::state::AppState;
use crate::ui::styles;

#[derive(Debug, Default)]
pub struct SlideList {}

impl StatefulWidget for &SlideList {
    type State = AppState;

    #[allow(clippy::cast_possible_truncation)]
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let selected = state.current_slide_id;

        // Create list items for each slide
        let items: Vec<ListItem> = state
            .talk
            .titles
            .iter()
            .enumerate()
            .map(|(index, text)| {
                build_list_item(selected == Some(SlideId::new(index)), index + 1, text)
            })
            .collect();

        let block = Block::bordered()
            .title(Line::from(" Slides "))
            .border_set(border::THICK);

        let mut list_state = ListState::default().with_selected(selected.map(SlideId::index));
        let list = List::new(items).block(block);
        StatefulWidget::render(list, area, buf, &mut list_state);
    }
}

fn build_list_item(current: bool, number: usize, title: &str) -> ListItem<'_> {
    // let truncated_title = content_renderer::truncate_text(title, 25);
    let content = format!("{number:2}. {title}");
    let style = if current {
        styles::list::CURRENT_SLIDE_STYLE
    } else {
        styles::list::NORMAL_SLIDE_STYLE
    };

    ListItem::new(Line::from(Span::styled(content, style)))
}
