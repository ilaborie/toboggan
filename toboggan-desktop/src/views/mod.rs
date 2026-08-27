mod content;
mod footer;
mod help;
pub(crate) mod sidebar;
mod slide;

use iced::widget::{center, column, container, mouse_area, opaque, row, stack};
use iced::{Element, Length};

use crate::constants::{
    FONT_SIZE_MEDIUM, PADDING_CONTAINER, PADDING_SLIDE_CONTENT, SPACING_MEDIUM,
};
use crate::message::Message;
use crate::state::AppState;
use crate::styles;
use crate::widgets::create_text;

pub(crate) fn main_view(state: &AppState) -> Element<'_, Message> {
    let mut layers: Vec<Element<'_, Message>> = vec![presentation_view(state)];

    // Layered rather than swapped. The help used to *replace* the presentation,
    // so the one moment a reader most wants to check a key against what is on
    // screen — "which key was next step again?" — was the one moment the screen
    // was gone.
    if state.show_help {
        layers.push(overlay(help::view(), Message::ToggleHelp));
    }

    if let Some(error) = &state.error_message {
        layers.push(
            container(
                container(create_text(error, FONT_SIZE_MEDIUM))
                    .style(styles::error_container())
                    .padding(PADDING_CONTAINER),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_bottom(Length::Fill)
            .padding(PADDING_CONTAINER)
            .into(),
        );
    }

    stack(layers)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Floats `panel` over whatever is already drawn, dimmed, with a click outside
/// it sending `dismiss`.
///
/// `opaque` twice, deliberately: the outer one stops a click reaching the deck
/// behind, the inner one stops a click *inside* the panel counting as a click
/// outside it.
fn overlay(panel: Element<'_, Message>, dismiss: Message) -> Element<'_, Message> {
    opaque(
        mouse_area(
            container(center(opaque(panel)))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(styles::scrim()),
        )
        .on_press(dismiss),
    )
}

fn presentation_view(state: &AppState) -> Element<'_, Message> {
    let mut layout = row![];

    if state.show_sidebar {
        layout = layout.push(sidebar::view(state));
    }

    // Resolved once and handed down: `view` is given the state, never the
    // theme, so the three places that wanted a palette each reached for
    // `Theme::Dark` instead.
    let theme = state.theme();
    let main_area =
        column![slide::view(state, &theme), footer::view(state)].spacing(SPACING_MEDIUM);

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
