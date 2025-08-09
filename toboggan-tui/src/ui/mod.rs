use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use toboggan_core::{Content, Slide};

use crate::state::{AppState, ConnectionStatus};

pub fn render_ui(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Control panel
        ])
        .split(f.area());

    render_status_bar(f, chunks[0], state);
    render_main_content(f, chunks[1], state);
    render_control_panel(f, chunks[2]);

    if state.show_help {
        render_help_overlay(f);
    }

    if let Some(error) = &state.error_message {
        render_error_popup(f, error);
    }
}

fn render_status_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let status_color = match &state.connection_status {
        ConnectionStatus::Connected => Color::Green,
        ConnectionStatus::Connecting => Color::Yellow,
        ConnectionStatus::Disconnected => Color::Red,
        ConnectionStatus::Error(_) => Color::Red,
    };

    let status_text = match &state.connection_status {
        ConnectionStatus::Connected => "Connected",
        ConnectionStatus::Connecting => "Connecting...",
        ConnectionStatus::Disconnected => "Disconnected",
        ConnectionStatus::Error(msg) => msg,
    };

    let slide_info = if let Some(slide_id) = &state.current_slide {
        format!(" | Slide: {slide_id:?}")
    } else {
        " | No slide".to_string()
    };

    let content = Line::from(vec![
        Span::styled("Status: ", Style::default()),
        Span::styled(status_text, Style::default().fg(status_color)),
        Span::raw(slide_info),
    ]);

    let paragraph = Paragraph::new(content).style(Style::default().bg(Color::DarkGray));
    f.render_widget(paragraph, area);
}

fn render_main_content(f: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title("Slide Content")
        .borders(Borders::ALL);

    if let Some(slide_id) = &state.current_slide {
        if let Some(slide) = state.slides.get(slide_id) {
            render_slide_content(f, area, slide);
        } else {
            let loading = Paragraph::new("Loading slide...")
                .block(block)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(loading, area);
        }
    } else {
        let waiting = Paragraph::new("Waiting for presentation...")
            .block(block)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(waiting, area);
    }
}

fn render_slide_content(f: &mut Frame, area: Rect, slide: &Slide) {
    let title = content_to_string(&slide.title).unwrap_or_else(|| "Untitled".to_string());

    let body = content_to_string(&slide.body).unwrap_or_else(|| "No content".to_string());

    let content = format!("{}\n\n{}", title, body);

    let block = Block::default()
        .title(format!("Slide ({:?})", slide.kind))
        .borders(Borders::ALL);

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_control_panel(f: &mut Frame, area: Rect) {
    let block = Block::default().title("Controls").borders(Borders::ALL);

    let controls = vec![
        Line::from("F:First  P:Previous  N:Next  L:Last  Space:Play/Pause"),
        Line::from("H:Help  R:Reconnect  C:Clear Error  Q:Quit"),
    ];

    let paragraph = Paragraph::new(controls)
        .block(block)
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(paragraph, area);
}

fn render_help_overlay(f: &mut Frame) {
    let area = centered_rect(60, 70, f.area());
    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from("Keyboard Shortcuts"),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  F / Home     - First slide"),
        Line::from("  P / ←        - Previous slide"),
        Line::from("  N / → / Space - Next slide"),
        Line::from("  L / End      - Last slide"),
        Line::from(""),
        Line::from("Presentation:"),
        Line::from("  Space        - Play/Pause toggle"),
        Line::from(""),
        Line::from("Application:"),
        Line::from("  H / ?        - Toggle help panel"),
        Line::from("  R            - Reconnect WebSocket"),
        Line::from("  C            - Clear error message"),
        Line::from("  Q / Ctrl+C   - Quit application"),
        Line::from(""),
        Line::from("Press H or ? to close this help"),
    ];

    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, area);
}

fn render_error_popup(f: &mut Frame, error: &str) {
    let area = centered_rect(50, 20, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("Error")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Red).fg(Color::White));

    let paragraph = Paragraph::new(format!("{error}\n\nPress C to clear"))
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, rect: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(rect);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])
        .get(1)
        .copied()
        .unwrap_or(popup_layout[1])
}

fn content_to_string(content: &Content) -> Option<String> {
    match content {
        Content::Empty => None,
        Content::Text { text } => Some(text.clone()),
        Content::Html { raw, alt } => alt.clone().or_else(|| Some(raw.clone())),
        Content::IFrame { url } => Some(format!("IFrame: {url}")),
        Content::Term { .. } => Some("Terminal content (not supported in TUI)".to_string()),
        Content::Grid { contents, .. } => {
            let texts: Vec<String> = contents.iter().filter_map(content_to_string).collect();
            if texts.is_empty() {
                None
            } else {
                Some(texts.join(" "))
            }
        }
    }
}
