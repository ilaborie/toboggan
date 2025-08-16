use ratatui::layout::Flex;
use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use tui_logger::TuiLoggerWidget;

use crate::state::{AppDialog, AppState};
use crate::ui::styles::layout;
use crate::ui::widgets::{
    CurrentSlide, HelpPanel, NextSlidePreview, ProgressBar, SlideList, SpeakerNotes, TitleBar,
};

pub mod styles;
pub mod widgets;

#[derive(Default)]
pub struct PresenterComponents {
    title_bar: TitleBar,
    progress_bar: ProgressBar,
    slide_list: SlideList,
    current_slide: CurrentSlide,
    next_slide_preview: NextSlidePreview,
    speaker_notes: SpeakerNotes,
    help_panel: HelpPanel,
}

impl PresenterComponents {}

impl StatefulWidget for &PresenterComponents {
    type State = AppState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                layout::TOP_BAR,
                layout::MAIN_CONTENT,
                layout::SPEAKER_NOTES,
                // layout::LOG_PANEL,     // Log panel
            ]);
        let [top_area, content_area, notes_area] = main_layout.areas(area);

        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(layout::SLIDE_LIST_PERCENTAGE),
                Constraint::Percentage(layout::CURRENT_SLIDE_PERCENTAGE),
                Constraint::Percentage(layout::NEXT_SLIDE_PERCENTAGE),
            ]);
        let [slides_area, current_area, next_area] = content_layout.areas(content_area);

        // Topbar - split into title and progress areas
        let top_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(layout::CONTROL_TITLE_MIN_WIDTH),
                Constraint::Length(layout::CONTROL_PROGRESS_WIDTH),
            ]);
        let [title_area, progress_area] = top_layout.areas(top_area);

        (&self.title_bar).render(title_area, buf, state);
        (&self.progress_bar).render(progress_area, buf, state);

        // Main content area - 3 columns
        (&self.slide_list).render(slides_area, buf, state);
        (&self.current_slide).render(current_area, buf, state);
        (&self.next_slide_preview).render(next_area, buf, state);

        // Notes
        (&self.speaker_notes).render(notes_area, buf, state);

        // Dialogs
        match &state.dialog {
            AppDialog::Help => {
                let area = popup_area(area, 52, 22);
                Clear.render(area, buf);
                (&self.help_panel).render(area, buf);
            }
            AppDialog::Log => {
                // let area = popup_area(area, 80, 40);
                Clear.render(area, buf);
                let block = Block::bordered().title(" Logs").border_set(border::ROUNDED);
                TuiLoggerWidget::default()
                    .block(block)
                    .style_debug(styles::log::DEBUG)
                    .style_info(styles::log::INFO)
                    .style_warn(styles::log::WARN)
                    .style_error(styles::log::ERROR)
                    .render(area, buf);
            }
            AppDialog::Error(error) => {
                let area = popup_area(area, 60, 8);
                Clear.render(area, buf);
                let block = Block::bordered()
                    .title(" 🚨 Error ")
                    .border_set(border::ROUNDED);
                let content = Line::from(Span::styled(error, styles::colors::RED));
                Paragraph::new(content)
                    .block(block)
                    .wrap(Wrap { trim: true })
                    .render(area, buf);
            }
            AppDialog::None => {}
        }
    }
}

fn popup_area(area: Rect, x: u16, y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Length(x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
