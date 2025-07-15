use ratatui::style::{Modifier, Style};

pub mod colors {
    use ratatui::style::Color;

    pub const WHITE: Color = Color::White;
    pub const BLACK: Color = Color::Black;
    pub const GRAY: Color = Color::Gray;

    pub const GREEN: Color = Color::Green;
    pub const RED: Color = Color::Red;
    pub const YELLOW: Color = Color::Yellow;
    pub const BLUE: Color = Color::Blue;
    pub const CYAN: Color = Color::Cyan;
    pub const MAGENTA: Color = Color::Magenta;
}

pub mod action {
    use super::{Modifier, Style, colors};

    pub const KEY: Style = Style::new().fg(colors::CYAN);
    pub const DESCRIPTION: Style = Style::new().fg(colors::GRAY);
    pub const TITLE: Style = Style::new().add_modifier(Modifier::BOLD);
}

pub mod log {
    use super::{Style, colors};

    pub const DEBUG: Style = Style::new().fg(colors::GREEN);
    pub const INFO: Style = Style::new().fg(colors::BLUE);
    pub const WARN: Style = Style::new().fg(colors::YELLOW);
    pub const ERROR: Style = Style::new().fg(colors::RED);
}

/// Talk state styles
pub mod state {
    use super::{Style, colors};

    pub const PAUSED: Style = Style::new().fg(colors::YELLOW);
    pub const RUNNING: Style = Style::new().fg(colors::GRAY);
    pub const DONE: Style = Style::new().fg(colors::GREEN);
}

/// Slide kind specific styles  
pub mod slide {
    use ratatui::text::Span;
    use toboggan_core::SlideKind;

    use super::{Modifier, Style, colors};

    pub const COVER_STYLE: Style = Style::new().fg(colors::YELLOW).add_modifier(Modifier::BOLD);

    pub const PART_STYLE: Style = Style::new()
        .fg(colors::MAGENTA)
        .add_modifier(Modifier::BOLD);

    pub const STANDARD_STYLE: Style = Style::new().fg(colors::WHITE).add_modifier(Modifier::BOLD);

    /// Get style and indicator for a slide kind
    #[must_use]
    pub fn get_slide_kind_span<'a>(kind: SlideKind) -> Span<'a> {
        match kind {
            SlideKind::Cover => Span::styled(" [COVER]", COVER_STYLE),
            SlideKind::Part => Span::styled(" [PART]", PART_STYLE),
            SlideKind::Standard => Span::styled("", STANDARD_STYLE),
        }
    }
}

/// List and selection styles
pub mod list {
    use super::{Modifier, Style, colors};

    pub const CURRENT_SLIDE_STYLE: Style = Style::new()
        .fg(colors::BLACK)
        .bg(colors::YELLOW)
        .add_modifier(Modifier::BOLD);

    pub const NORMAL_SLIDE_STYLE: Style = Style::new().fg(colors::WHITE);
}

/// General UI styles
pub mod ui {
    use super::{Modifier, Style, colors};

    pub const NO_CONTENT_STYLE: Style =
        Style::new().fg(colors::GRAY).add_modifier(Modifier::ITALIC);
}

/// Layout constraints commonly used
pub mod layout {
    use ratatui::layout::Constraint;

    // Control bar layout
    pub const CONTROL_BAR_HEIGHT: u16 = 3;
    pub const SPEAKER_NOTES_HEIGHT: u16 = 16;
    // Control bar horizontal layout
    pub const CONTROL_TITLE_MIN_WIDTH: u16 = 20;
    pub const CONTROL_PROGRESS_WIDTH: u16 = 30;

    // Main content area percentages
    pub const SLIDE_LIST_PERCENTAGE: u16 = 20;
    pub const CURRENT_SLIDE_PERCENTAGE: u16 = 50;
    pub const NEXT_SLIDE_PERCENTAGE: u16 = 30;

    // Common constraints
    pub const TOP_BAR: Constraint = Constraint::Length(CONTROL_BAR_HEIGHT);
    pub const MAIN_CONTENT: Constraint = Constraint::Min(8);
    pub const SPEAKER_NOTES: Constraint = Constraint::Length(SPEAKER_NOTES_HEIGHT);
}
