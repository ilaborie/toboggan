mod content;
mod footer;
mod help;
mod sidebar;
mod slide;

use iced::widget::{column, container, row};
use iced::{Element, Length};

use crate::constants::{
    FONT_SIZE_MEDIUM, PADDING_CONTAINER, PADDING_SLIDE_CONTENT, SPACING_MEDIUM,
};
use crate::message::Message;
use crate::state::AppState;
use crate::styles;
use crate::widgets::create_text;

pub fn main_view(state: &AppState) -> Element<'_, Message> {
    let main_content = if state.show_help {
        help::view()
    } else {
        presentation_view(state)
    };

    if let Some(error) = &state.error_message {
        container(column![
            main_content,
            container(create_text(error, FONT_SIZE_MEDIUM))
                .style(styles::error_container())
                .padding(PADDING_CONTAINER)
                .width(Length::Fill)
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        main_content
    }
}

fn presentation_view(state: &AppState) -> Element<'_, Message> {
    let mut layout = row![];

    if state.show_sidebar {
        layout = layout.push(sidebar::view(state));
    }

    let main_area = column![slide::view(state), footer::view(state),].spacing(SPACING_MEDIUM);

    layout = layout.push(
        container(main_area)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(PADDING_SLIDE_CONTENT),
    );

    container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
