use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::actions::{ActionDetails, HELP_GROUPS};
use crate::constants::{FONT_SIZE_LARGE, FONT_SIZE_MEDIUM, FONT_SIZE_SMALL};
use crate::message::Message;
use crate::styles;
use crate::widgets::{create_muted_text, create_text};

/// Width of the key column, wide enough for the longest label pair.
const KEYS_WIDTH: f32 = 180.0;

/// How wide the panel is allowed to grow. A help list read across a 4K window
/// is one long line per shortcut with the description a metre from its key.
const PANEL_WIDTH: f32 = 640.0;

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

    container(
        scrollable(help_content.spacing(4.0).padding(30.0))
            .height(Length::Shrink)
            .anchor_top(),
    )
    .max_width(PANEL_WIDTH)
    .style(styles::overlay_panel())
    .into()
}
