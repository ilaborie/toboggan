use iced::widget::{column, container};
use iced::{Element, Length};

use crate::constants::{FONT_SIZE_LARGE, FONT_SIZE_MEDIUM, FONT_SIZE_SMALL, FONT_SIZE_TITLE};
use crate::message::Message;
use crate::widgets::{create_muted_text, create_text};

pub fn view() -> Element<'static, Message> {
    let help_content = column![
        create_text("Toboggan Desktop Help", 24.0),
        create_text("", FONT_SIZE_SMALL),
        create_text("Keyboard Shortcuts:", FONT_SIZE_TITLE),
        create_text("", 8.0),
        create_text("Step Navigation:", FONT_SIZE_LARGE),
        create_text("  Space / ↓    Next step", FONT_SIZE_MEDIUM),
        create_text("  ↑            Previous step", FONT_SIZE_MEDIUM),
        create_text("", 8.0),
        create_text("Slide Navigation:", FONT_SIZE_LARGE),
        create_text("  →            Next slide", FONT_SIZE_MEDIUM),
        create_text("  ←            Previous slide", FONT_SIZE_MEDIUM),
        create_text("  Home         First slide", FONT_SIZE_MEDIUM),
        create_text("  End          Last slide", FONT_SIZE_MEDIUM),
        create_text("", 8.0),
        create_text("Presentation Control:", FONT_SIZE_LARGE),
        create_text("  p / P        Pause presentation", FONT_SIZE_MEDIUM),
        create_text("  r / R        Resume presentation", FONT_SIZE_MEDIUM),
        create_text("  b / B        Blink (bell/notification)", FONT_SIZE_MEDIUM),
        create_text("", 8.0),
        create_text("View:", FONT_SIZE_LARGE),
        create_text("  h / ?        Toggle this help", FONT_SIZE_MEDIUM),
        create_text("  s            Toggle sidebar", FONT_SIZE_MEDIUM),
        create_text("  F11          Toggle fullscreen", FONT_SIZE_MEDIUM),
        create_text("  Escape       Close help/error", FONT_SIZE_MEDIUM),
        create_text("", 8.0),
        create_text("Application:", FONT_SIZE_LARGE),
        create_text("  Cmd+Q        Quit application", FONT_SIZE_MEDIUM),
        create_text("", FONT_SIZE_SMALL),
        create_muted_text("Press Escape to close this help"),
    ]
    .spacing(4.0)
    .padding(30.0);

    container(help_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.98, 0.98, 0.98, 0.95,
            ))),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}
