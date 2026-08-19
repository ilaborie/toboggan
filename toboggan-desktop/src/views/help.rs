use iced::widget::{column, container, row, text};
use iced::{Element, Length};

use crate::actions::{ActionDetails, HELP_GROUPS};
use crate::constants::{FONT_SIZE_LARGE, FONT_SIZE_MEDIUM, FONT_SIZE_SMALL};
use crate::message::Message;
use crate::widgets::{create_muted_text, create_text};

/// Width of the key column, wide enough for the longest label pair.
const KEYS_WIDTH: f32 = 180.0;

pub(super) fn view() -> Element<'static, Message> {
    let mut help_content = column![create_text("Toboggan Desktop Help", 24.0)];

    for (group, actions) in HELP_GROUPS {
        help_content = help_content.push(create_text("", FONT_SIZE_SMALL));
        help_content = help_content.push(create_text(group, FONT_SIZE_LARGE));
        for action in *actions {
            let ActionDetails { keys, description } = action.details();
            help_content = help_content.push(
                row![
                    container(text(keys.join(" / ")).size(FONT_SIZE_MEDIUM)).width(KEYS_WIDTH),
                    text(description).size(FONT_SIZE_MEDIUM),
                ]
                .padding([0.0, 12.0]),
            );
        }
    }

    help_content = help_content.push(create_text("", FONT_SIZE_SMALL));
    help_content = help_content.push(create_muted_text("Press Escape to close this help"));

    container(help_content.spacing(4.0).padding(30.0))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme: &iced::Theme| container::Style {
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
