use iced::widget::container;
use iced::{Background, Border, Theme};

use crate::constants::{BORDER_RADIUS, BORDER_WIDTH};

// Container styles
pub(crate) fn card_container() -> impl Fn(&Theme) -> container::Style {
    |theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(palette.background.base.color)),
            border: Border {
                color: palette.background.strong.color,
                width: BORDER_WIDTH,
                radius: BORDER_RADIUS.into(),
            },
            ..Default::default()
        }
    }
}

pub(crate) fn footer_container() -> impl Fn(&Theme) -> container::Style {
    |theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(palette.background.weak.color)),
            border: Border {
                color: palette.background.strong.color,
                width: BORDER_WIDTH,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    }
}

pub(crate) fn error_container() -> impl Fn(&Theme) -> container::Style {
    |theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(palette.danger.strong.color)),
            text_color: Some(palette.danger.strong.text),
            border: Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: BORDER_RADIUS.into(),
            },
            ..Default::default()
        }
    }
}

pub(crate) fn preview_container() -> impl Fn(&Theme) -> container::Style {
    |theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(palette.background.weak.color)),
            border: Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: BORDER_RADIUS.into(),
            },
            ..Default::default()
        }
    }
}

/// Dims whatever the overlay is covering.
///
/// Derived from the theme's own background rather than a literal, so it dims in
/// both directions: a hardcoded near-white scrim is invisible over a light deck
/// and blinding over a dark one.
pub(crate) fn scrim() -> impl Fn(&Theme) -> container::Style {
    |theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(
                palette.background.base.color.scale_alpha(0.85),
            )),
            ..Default::default()
        }
    }
}

/// The panel an overlay draws inside.
///
/// The help panel used to set its own near-white background from a closure that
/// took `&Theme` and ignored it — with text left at the theme's foreground,
/// which under the only theme the app had is also near-white. Pressing `h`, the
/// first thing anyone does, gave white on white.
pub(crate) fn overlay_panel() -> impl Fn(&Theme) -> container::Style {
    |theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(palette.background.weak.color)),
            text_color: Some(palette.background.base.text),
            border: Border {
                color: palette.background.strong.color,
                width: BORDER_WIDTH,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    }
}
