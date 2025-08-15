use ratatui::prelude::*;

use crate::state::AppState;
use crate::ui::styles::layout;
use crate::ui::widgets::{ProgressBar, TitleBar};

#[derive(Debug, Default)]
pub struct NavBar {}

impl StatefulWidget for &NavBar {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(layout::CONTROL_TITLE_MIN_WIDTH),
                Constraint::Length(layout::CONTROL_PROGRESS_WIDTH),
            ]);
        let [title_area, progress_area] = layout.areas(area);

        let title = TitleBar::default();
        title.render(title_area, buf, state);

        let progress = ProgressBar::default();
        progress.render(progress_area, buf, state);
    }
}
