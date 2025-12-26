use std::cmp::Ordering;

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

        // Build step indicators if available
        let step_indicators = step_indicators_spans(state);

        let mut title_spans = vec![
            Span::raw(" Slide "),
            Span::raw(format!("{current:02}")),
            Span::raw("/"),
            Span::raw(format!("{count:02}")),
            Span::raw(" "),
        ];
        title_spans.extend(step_indicators);
        let title = Line::from(title_spans);

        let bottom = match state.presentation_state {
            State::Init => Line::default(),
            State::Running { .. } => Line::from(vec![
                Span::raw(" "),
                Span::styled("Running", styles::state::RUNNING),
                Span::raw(" "),
            ]),
            State::Done { .. } => Line::from(vec![
                Span::raw(" "),
                Span::styled("🎉 Done", styles::state::DONE),
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

/// Build step indicator spans (●/○) for the current slide
fn step_indicators_spans(state: &AppState) -> Vec<Span<'static>> {
    let Some((current_step, step_count)) = state.step_info() else {
        return vec![];
    };

    if step_count == 0 {
        return vec![];
    }

    let mut spans = Vec::with_capacity(step_count);
    for step in 0..step_count {
        let (circle, style) = match step.cmp(&current_step) {
            Ordering::Less => ("●", styles::step::DONE),     // Done
            Ordering::Equal => ("●", styles::step::CURRENT), // Current
            Ordering::Greater => ("○", styles::step::REMAINING), // Remaining
        };
        spans.push(Span::styled(circle, style));
    }

    spans
}
