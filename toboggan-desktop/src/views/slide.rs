use iced::widget::{column, container, scrollable};
use iced::{Element, Length};
use toboggan_core::Content;

use super::content;
use crate::constants::{
    COLOR_MUTED, FONT_SIZE_LARGE, PADDING_CONTAINER, PADDING_SLIDE_CONTENT, SLIDE_NOTES_HEIGHT,
    SLIDE_NOTES_SCROLL_HEIGHT, SPACING_LARGE, SPACING_SMALL,
};
use crate::message::Message;
use crate::state::AppState;
use crate::styles;

pub fn view(state: &AppState) -> Element<'_, Message> {
    if let Some(slide) = state.current_slide() {
        let mut content_column = column![]
            .spacing(SPACING_LARGE)
            .padding(PADDING_SLIDE_CONTENT);

        // Slide kind indicator
        let kind_text = match slide.kind {
            toboggan_core::SlideKind::Cover => "COVER",
            toboggan_core::SlideKind::Part => "PART",
            toboggan_core::SlideKind::Standard => "",
        };

        if !kind_text.is_empty() {
            content_column =
                content_column.push(iced::widget::text(kind_text).size(12).color(COLOR_MUTED));
        }

        // Title
        if !matches!(&slide.title, Content::Text { text } if text.is_empty()) {
            let title_content = content::render_content(&slide.title);
            content_column = content_column.push(iced::widget::text(title_content).size(32));
        }

        // Body
        if !matches!(&slide.body, Content::Text { text } if text.is_empty()) {
            content_column = content_column.push(
                scrollable(
                    container(content::render_content_element(&slide.body))
                        .width(Length::Fill)
                        .padding(PADDING_CONTAINER),
                )
                .height(Length::FillPortion(3)),
            );
        }

        // Speaker Notes
        if !matches!(&slide.notes, Content::Text { text } if text.is_empty()) {
            content_column = content_column.push(
                container(
                    column![
                        iced::widget::text("Speaker Notes")
                            .size(FONT_SIZE_LARGE)
                            .color(COLOR_MUTED),
                        container(
                            scrollable(
                                container(content::render_content_element(&slide.notes))
                                    .padding(PADDING_CONTAINER)
                                    .width(Length::Fill)
                            )
                            .height(Length::Fixed(SLIDE_NOTES_SCROLL_HEIGHT))
                        )
                        .height(Length::Fixed(SLIDE_NOTES_HEIGHT))
                        .style(styles::preview_container())
                    ]
                    .spacing(SPACING_SMALL),
                )
                .width(Length::Fill),
            );
        }

        container(content_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill)
            .into()
    } else {
        container(
            iced::widget::text("No slide loaded")
                .size(24)
                .color(COLOR_MUTED),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(iced::Length::Fill)
        .center_y(iced::Length::Fill)
        .into()
    }
}
