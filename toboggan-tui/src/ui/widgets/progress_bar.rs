use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, LineGauge};
use toboggan_core::State;

use crate::state::AppState;
use crate::ui::styles;

#[derive(Debug, Default)]
pub struct ProgressBar {}

impl StatefulWidget for &ProgressBar {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let current = state.current();
        let count = state.count();

        let title = Line::from(vec![
            Span::raw(" Slide "),
            Span::raw(format!("{current:02}")),
            Span::raw("/"),
            Span::raw(format!("{count:02}")),
            Span::raw(" "),
        ]);

        let bottom = match state.presentation_state {
            State::Init => Line::default(),
            State::Paused { total_duration, .. } => Line::from(vec![
                Span::raw(" "),
                Span::styled("Paused", styles::state::PAUSED),
                Span::raw(" - "),
                Span::raw(format!("{total_duration}")),
                Span::raw(" "),
            ]),
            State::Running { total_duration, .. } => Line::from(vec![
                Span::raw(" "),
                Span::styled("Running", styles::state::RUNNING),
                Span::raw(" - "),
                Span::raw(format!("{total_duration}")),
                Span::raw(" "),
            ]),
            State::Done { total_duration, .. } => Line::from(vec![
                Span::raw(" "),
                Span::styled("🎉 Done", styles::state::DONE),
                Span::raw(" - "),
                Span::raw(format!("{total_duration}")),
                Span::raw(" "),
            ]),
        };
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(bottom.centered())
            .border_set(border::PLAIN);

        #[allow(clippy::cast_precision_loss)]
        LineGauge::default()
            .line_set(symbols::line::THICK)
            .ratio(current as f64 / count as f64)
            .filled_style(styles::colors::BLUE)
            .unfilled_style(styles::colors::BLACK)
            .block(block)
            .render(area, buf);
    }
}
