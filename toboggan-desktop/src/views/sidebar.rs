use iced::widget::{Column, button, column, container, row, scrollable, text};
use iced::{Element, Length, Padding, Theme};
use toboggan_core::Command;

use super::content;
use crate::constants::{
    FONT_SIZE_MEDIUM, FONT_SIZE_SMALL, PADDING_CONTAINER, SPACING_MEDIUM, SPACING_SMALL,
};
use crate::message::Message;
use crate::slide_list::{self, Row};
use crate::state::AppState;
use crate::styles;
use crate::widgets::{create_body_text, create_title_text};

/// The scrollable holding the slide list, so `app` can snap it to the slide the
/// deck is on. A `const` id rather than one per render: it names the widget, and
/// the widget is the same one every frame.
pub(crate) const SLIDE_LIST_ID: iced::widget::Id = iced::widget::Id::new("sidebar.slides");

/// How far each level of nesting indents a row.
const INDENT: f32 = 12.0;

pub(super) fn view(state: &AppState) -> Element<'_, Message> {
    let mut sidebar_content = column![create_title_text("Slides")]
        .spacing(SPACING_MEDIUM)
        .padding(PADDING_CONTAINER);

    if state.talk.is_some() && !state.slides.is_empty() {
        let list = slide_list::rows(&state.slides, state.current_slide)
            .into_iter()
            .fold(Column::new().spacing(SPACING_SMALL), |list, row| {
                list.push(entry(&row))
            });

        sidebar_content = sidebar_content.push(
            scrollable(list)
                .id(SLIDE_LIST_ID)
                .height(Length::Fill)
                .anchor_top(),
        );
    }

    container(sidebar_content)
        .width(Length::Fixed(250.0))
        .height(Length::Fill)
        .style(styles::card_container())
        .into()
}

/// One row of the list.
fn entry(row: &Row) -> Element<'static, Message> {
    let label = if row.is_part() {
        text(row.label.clone()).size(FONT_SIZE_MEDIUM)
    } else {
        text(row.label.clone()).size(FONT_SIZE_SMALL)
    };

    let content = row![
        text(row.number.to_string())
            .size(FONT_SIZE_SMALL)
            .style(text::secondary),
        label,
    ]
    .spacing(SPACING_SMALL)
    .align_y(iced::Alignment::Center);

    button(content)
        .on_press(Message::SendCommand(Command::GoTo { slide: row.id }))
        .width(Length::Fill)
        .padding(
            Padding::new(4.0)
                .right(8.0)
                .left(8.0 + f32::from(row.depth) * INDENT),
        )
        .style(if row.is_current {
            |theme: &Theme, status| button::primary(theme, status)
        } else if row.is_part() {
            |theme: &Theme, status| button::secondary(theme, status)
        } else {
            |theme: &Theme, status| button::text(theme, status)
        })
        .into()
}

/// The next slide's number and title, for the pane that shows what is coming.
pub(super) fn next_slide_preview(state: &AppState) -> Option<Element<'_, Message>> {
    let next = state.next_slide()?;
    let number = state.current_slide.map_or(0, |id| id.display_number() + 1);
    let title = if next.title.is_blank() {
        format!("Slide {number}")
    } else {
        content::render_content(&next.title)
    };

    Some(
        column![
            create_body_text("Next"),
            container(text(format!("{number}. {title}")).size(FONT_SIZE_MEDIUM))
                .padding(SPACING_SMALL)
                .width(Length::Fill)
                .style(styles::preview_container()),
        ]
        .spacing(SPACING_SMALL)
        .into(),
    )
}
