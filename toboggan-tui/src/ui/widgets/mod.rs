use ratatui::prelude::*;

mod top_bar;

pub use self::top_bar::NavBar;
use crate::events::AppAction;
use crate::ui::styles;

mod title_bar;
pub use self::title_bar::TitleBar;

mod progress_bar;
pub use self::progress_bar::ProgressBar;

mod slide_list;
pub use self::slide_list::SlideList;

mod current_slide;
pub use self::current_slide::CurrentSlide;

mod next_slide_preview;
pub use self::next_slide_preview::NextSlidePreview;

mod speaker_notes;
pub use self::speaker_notes::SpeakerNotes;

mod help_panel;
pub use self::help_panel::HelpPanel;

fn line_from_actions(actions: &[AppAction]) -> Line<'_> {
    if actions.is_empty() {
        return Line::default();
    }

    let mut spans = vec![Span::raw(" ")];
    let mut first = true;
    for action in actions {
        if first {
            first = false;
        } else {
            spans.push(Span::raw(" · "));
        }

        let key = action.key();
        spans.push(Span::styled(format!("[{key}] "), styles::action::KEY));
        spans.push(Span::styled(
            action.to_string(),
            styles::action::DESCRIPTION,
        ));
    }
    spans.push(Span::raw("  "));

    Line::from(spans)
}
