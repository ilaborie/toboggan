use iced::widget::{self, column, container, markdown, scrollable};
use iced::{Element, Length, Theme};
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
        let cached_md = state.current_markdown();

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
                content_column.push(widget::text(kind_text).size(12.0).color(COLOR_MUTED));
        }

        // Title
        if !matches!(&slide.title, Content::Text { text } if text.is_empty()) {
            let title_content = content::render_content(&slide.title);
            content_column = content_column.push(widget::text(title_content).size(32.0));
        }

        // Body with markdown rendering
        if !matches!(&slide.body, Content::Text { text } if text.is_empty()) {
            let body_element: Element<'_, Message> = if let Some(md) = cached_md {
                let settings = markdown::Settings::with_style(markdown::Style::from_palette(
                    Theme::Dark.palette(),
                ));
                markdown::view(&md.body_items, settings).map(Message::LinkClicked)
            } else {
                content::render_content_element(&slide.body)
            };

            content_column = content_column.push(
                scrollable(
                    container(body_element)
                        .width(Length::Fill)
                        .padding(PADDING_CONTAINER),
                )
                .height(Length::FillPortion(3))
                .anchor_top(),
            );
        }

        // Speaker Notes with markdown rendering
        if !matches!(&slide.notes, Content::Text { text } if text.is_empty()) {
            let notes_element: Element<'_, Message> = if let Some(md) = cached_md {
                let settings = markdown::Settings::with_style(markdown::Style::from_palette(
                    Theme::Dark.palette(),
                ));
                markdown::view(&md.notes_items, settings).map(Message::LinkClicked)
            } else {
                content::render_content_element(&slide.notes)
            };

            content_column = content_column.push(
                container(
                    column![
                        iced::widget::text("Speaker Notes")
                            .size(FONT_SIZE_LARGE)
                            .color(COLOR_MUTED),
                        container(
                            scrollable(
                                container(notes_element)
                                    .padding(PADDING_CONTAINER)
                                    .width(Length::Fill)
                            )
                            .height(Length::Fixed(SLIDE_NOTES_SCROLL_HEIGHT))
                            .anchor_bottom()
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
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    } else {
        container(
            widget::text("No slide loaded")
                .size(24.0)
                .color(COLOR_MUTED),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}
