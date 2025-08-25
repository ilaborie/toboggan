use iced::widget::container;
use iced::{Background, Border, Theme};

use crate::constants::{BORDER_RADIUS, BORDER_WIDTH};

// Container styles
pub fn card_container() -> impl Fn(&Theme) -> container::Style {
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

pub fn footer_container() -> impl Fn(&Theme) -> container::Style {
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

pub fn error_container() -> impl Fn(&Theme) -> container::Style {
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

pub fn terminal_container() -> impl Fn(&Theme) -> container::Style {
    |theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(palette.secondary.strong.color)),
            border: Border {
                color: palette.success.base.color,
                width: BORDER_WIDTH,
                radius: BORDER_RADIUS.into(),
            },
            ..Default::default()
        }
    }
}

pub fn iframe_container() -> impl Fn(&Theme) -> container::Style {
    |theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(palette.primary.weak.color)),
            border: Border {
                color: palette.primary.strong.color,
                width: BORDER_WIDTH,
                radius: BORDER_RADIUS.into(),
            },
            ..Default::default()
        }
    }
}

pub fn preview_container() -> impl Fn(&Theme) -> container::Style {
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
