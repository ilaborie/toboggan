use ratatui::style::{Modifier, Style};

pub(crate) mod colors {
    use ratatui::style::Color;

    pub(crate) const WHITE: Color = Color::White;
    pub(crate) const BLACK: Color = Color::Black;
    pub(crate) const GRAY: Color = Color::Gray;

    pub(crate) const GREEN: Color = Color::Green;
    pub(crate) const RED: Color = Color::Red;
    pub(crate) const YELLOW: Color = Color::Yellow;
    pub(crate) const BLUE: Color = Color::Blue;
    pub(crate) const CYAN: Color = Color::Cyan;
    pub(crate) const MAGENTA: Color = Color::Magenta;
}

pub(crate) mod action {
    use super::{Modifier, Style, colors};

    pub(crate) const KEY: Style = Style::new().fg(colors::CYAN);
    pub(crate) const DESCRIPTION: Style = Style::new().fg(colors::GRAY);
    pub(crate) const TITLE: Style = Style::new().add_modifier(Modifier::BOLD);
}

pub(crate) mod log {
    use super::{Style, colors};

    pub(crate) const DEBUG: Style = Style::new().fg(colors::GREEN);
    pub(crate) const INFO: Style = Style::new().fg(colors::BLUE);
    pub(crate) const WARN: Style = Style::new().fg(colors::YELLOW);
    pub(crate) const ERROR: Style = Style::new().fg(colors::RED);
}

/// Talk state styles
pub(crate) mod state {
    use super::{Style, colors};

    pub(crate) const RUNNING: Style = Style::new().fg(colors::GRAY);
    pub(crate) const DONE: Style = Style::new().fg(colors::GREEN);
}

/// Step indicator styles
pub(crate) mod step {
    use super::{Style, colors};

    pub(crate) const DONE: Style = Style::new().fg(colors::WHITE);
    pub(crate) const CURRENT: Style = Style::new().fg(colors::CYAN);
    pub(crate) const REMAINING: Style = Style::new().fg(colors::GRAY);
}

/// Slide kind specific styles
pub(crate) mod slide {
    use ratatui::text::Span;
    use toboggan_core::SlideKind;

    use super::{Modifier, Style, colors};

    pub(crate) const COVER_STYLE: Style =
        Style::new().fg(colors::YELLOW).add_modifier(Modifier::BOLD);

    pub(crate) const PART_STYLE: Style = Style::new()
        .fg(colors::MAGENTA)
        .add_modifier(Modifier::BOLD);

    pub(crate) const STANDARD_STYLE: Style =
        Style::new().fg(colors::WHITE).add_modifier(Modifier::BOLD);

    /// Get style and indicator for a slide kind
    #[must_use]
    pub(crate) fn get_slide_kind_span<'a>(kind: SlideKind) -> Span<'a> {
        match kind {
            SlideKind::Cover => Span::styled(" [COVER]", COVER_STYLE),
            SlideKind::Part => Span::styled(" [PART]", PART_STYLE),
            SlideKind::Standard => Span::styled("", STANDARD_STYLE),
        }
    }
}

/// List and selection styles
pub(crate) mod list {
    use super::{Modifier, Style, colors};

    pub(crate) const CURRENT_SLIDE_STYLE: Style = Style::new()
        .fg(colors::BLACK)
        .bg(colors::YELLOW)
        .add_modifier(Modifier::BOLD);

    pub(crate) const NORMAL_SLIDE_STYLE: Style = Style::new().fg(colors::WHITE);
}

/// General UI styles
pub(crate) mod ui {
    use super::{Modifier, Style, colors};

    pub(crate) const NO_CONTENT_STYLE: Style =
        Style::new().fg(colors::GRAY).add_modifier(Modifier::ITALIC);
}

/// Alert kind and styles matching GFM alert types
pub(crate) mod alert {
    use super::{Modifier, Style, colors};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Kind {
        Note,
        Tip,
        Important,
        Warning,
        Caution,
    }

    impl Kind {
        pub(crate) fn from_marker(marker: &str) -> Option<Self> {
            match marker {
                "NOTE" => Some(Self::Note),
                "TIP" => Some(Self::Tip),
                "IMPORTANT" => Some(Self::Important),
                "WARNING" => Some(Self::Warning),
                "CAUTION" => Some(Self::Caution),
                _ => None,
            }
        }

        pub(crate) fn label(self) -> &'static str {
            match self {
                Self::Note => "ⓘ NOTE",
                Self::Tip => "✔ TIP",
                Self::Important => "❗ IMPORTANT",
                Self::Warning => "⚠ WARNING",
                Self::Caution => "⛔ CAUTION",
            }
        }

        pub(crate) fn label_style(self) -> Style {
            self.body_style().add_modifier(Modifier::BOLD)
        }

        pub(crate) fn body_style(self) -> Style {
            match self {
                Self::Note => Style::new().fg(colors::BLUE),
                Self::Tip => Style::new().fg(colors::GREEN),
                Self::Important => Style::new().fg(colors::MAGENTA),
                Self::Warning => Style::new().fg(colors::YELLOW),
                Self::Caution => Style::new().fg(colors::RED),
            }
        }
    }
}

/// Layout constraints commonly used
pub(crate) mod layout {
    use ratatui::layout::Constraint;

    // Control bar layout
    pub(crate) const CONTROL_BAR_HEIGHT: u16 = 3;
    pub(crate) const SPEAKER_NOTES_HEIGHT: u16 = 16;
    // Control bar horizontal layout
    pub(crate) const CONTROL_TITLE_MIN_WIDTH: u16 = 20;
    pub(crate) const CONTROL_PROGRESS_WIDTH: u16 = 30;

    // Main content area percentages
    pub(crate) const SLIDE_LIST_PERCENTAGE: u16 = 20;
    pub(crate) const CURRENT_SLIDE_PERCENTAGE: u16 = 50;
    pub(crate) const NEXT_SLIDE_PERCENTAGE: u16 = 30;

    // Common constraints
    pub(crate) const TOP_BAR: Constraint = Constraint::Length(CONTROL_BAR_HEIGHT);
    pub(crate) const MAIN_CONTENT: Constraint = Constraint::Min(8);
    pub(crate) const SPEAKER_NOTES: Constraint = Constraint::Length(SPEAKER_NOTES_HEIGHT);
}
