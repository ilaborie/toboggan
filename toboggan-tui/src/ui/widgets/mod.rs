use ratatui::prelude::*;

use crate::events::AppAction;
use crate::ui::styles;

mod title_bar;
pub(crate) use self::title_bar::TitleBar;

mod progress_bar;
pub(crate) use self::progress_bar::ProgressBar;

mod slide_list;
pub(crate) use self::slide_list::SlideList;

mod current_slide;
pub(crate) use self::current_slide::CurrentSlide;

mod next_slide_preview;
pub(crate) use self::next_slide_preview::NextSlidePreview;

mod speaker_notes;
pub(crate) use self::speaker_notes::SpeakerNotes;

mod help_panel;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Paragraph};

pub(crate) use self::help_panel::HelpPanel;

/// Helper function to render "no content" message
pub(crate) fn render_no_content(
    area: Rect,
    buf: &mut Buffer,
    message: &str,
    border_set: border::Set<'_>,
) {
    let title = Line::from(Span::styled(
        format!(" <{message}> "),
        styles::ui::NO_CONTENT_STYLE,
    ));
    let block = Block::bordered().title(title).border_set(border_set);
    Paragraph::new(vec![]).block(block).render(area, buf);
}

/// Helper function to convert content text to styled lines.
///
/// Recognizes GFM alert blocks (`> [!NOTE]`, `> [!TIP]`, etc.) and renders
/// them with a colored label and a `│`-prefixed body, matching the HTML output.
pub(crate) fn format_content_lines(content: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut current_alert: Option<styles::alert::Kind> = None;

    for line in content.lines() {
        if let Some(kind) = try_parse_alert_marker(line) {
            current_alert = Some(kind);
            lines.push(Line::from(Span::styled(kind.label(), kind.label_style())));
            continue;
        }

        if let Some(kind) = current_alert {
            if let Some(rest) = line.strip_prefix('>') {
                let body = rest.strip_prefix(' ').unwrap_or(rest);
                let style = kind.body_style();
                lines.push(Line::from(vec![
                    Span::styled("│ ", style),
                    Span::styled(body, style),
                ]));
                continue;
            }
            current_alert = None;
        }

        lines.push(Line::from(line));
    }

    lines
}

fn try_parse_alert_marker(line: &str) -> Option<styles::alert::Kind> {
    let rest = line.trim().strip_prefix('>')?;
    let marker_str = rest.trim().strip_prefix("[!")?.strip_suffix(']')?;
    styles::alert::Kind::from_marker(marker_str)
}

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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use ratatui::style::{Color, Modifier};

    use super::*;

    #[test]
    fn test_plain_lines_pass_through() {
        let lines = format_content_lines("hello\nworld");
        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines
                .first()
                .expect("line 0")
                .spans
                .first()
                .expect("span")
                .content,
            "hello"
        );
        assert_eq!(
            lines
                .get(1)
                .expect("line 1")
                .spans
                .first()
                .expect("span")
                .content,
            "world"
        );
    }

    #[test]
    fn test_note_alert_label_and_body() {
        let content = "> [!NOTE]\n> Some info";
        let lines = format_content_lines(content);
        assert_eq!(lines.len(), 2);
        let label_span = lines
            .first()
            .expect("label line")
            .spans
            .first()
            .expect("label span");
        assert_eq!(label_span.content, "ⓘ NOTE");
        assert!(label_span.style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(label_span.style.fg, Some(Color::Blue));
        let body_line = lines.get(1).expect("body line");
        assert_eq!(body_line.spans.first().expect("│ span").content, "│ ");
        assert_eq!(
            body_line.spans.first().expect("│ span").style.fg,
            Some(Color::Blue)
        );
        assert_eq!(
            body_line.spans.get(1).expect("body span").content,
            "Some info"
        );
    }

    #[test]
    fn test_warning_alert() {
        let content = "> [!WARNING]\n> Be careful";
        let lines = format_content_lines(content);
        assert_eq!(lines.len(), 2);
        let label_span = lines
            .first()
            .expect("label line")
            .spans
            .first()
            .expect("label span");
        assert_eq!(label_span.content, "⚠ WARNING");
        assert_eq!(label_span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_mixed_content_preserves_order() {
        let content = "intro\n\n> [!TIP]\n> A tip\n\noutro";
        let lines = format_content_lines(content);
        // intro, empty, label, body, empty, outro
        assert_eq!(lines.len(), 6);
        assert_eq!(
            lines
                .first()
                .expect("intro")
                .spans
                .first()
                .expect("span")
                .content,
            "intro"
        );
        assert_eq!(
            lines
                .get(2)
                .expect("label")
                .spans
                .first()
                .expect("span")
                .content,
            "✔ TIP"
        );
        assert_eq!(
            lines
                .get(5)
                .expect("outro")
                .spans
                .first()
                .expect("span")
                .content,
            "outro"
        );
    }

    #[test]
    fn test_unknown_alert_type_is_plain_blockquote() {
        let content = "> [!FOO]\n> text";
        let lines = format_content_lines(content);
        assert_eq!(
            lines
                .first()
                .expect("line 0")
                .spans
                .first()
                .expect("span")
                .content,
            "> [!FOO]"
        );
        assert_eq!(
            lines
                .get(1)
                .expect("line 1")
                .spans
                .first()
                .expect("span")
                .content,
            "> text"
        );
    }

    #[test]
    fn test_regular_blockquote_not_treated_as_alert() {
        let content = "> just a quote";
        let lines = format_content_lines(content);
        let span = lines.first().expect("line 0").spans.first().expect("span");
        assert_eq!(span.content, "> just a quote");
        assert_eq!(span.style.fg, None);
    }
}
