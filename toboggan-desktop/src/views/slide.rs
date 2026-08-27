use iced::widget::{self, column, container, markdown, scrollable};
use iced::{Element, Length, Theme};

use super::content;
use crate::constants::{
    COLOR_MUTED, FONT_SIZE_BODY, FONT_SIZE_LARGE, FONT_SIZE_NOTES, PADDING_CONTAINER,
    PADDING_SLIDE_CONTENT, PORTION_BODY, PORTION_NOTES, SPACING_LARGE, SPACING_SMALL,
};
use crate::message::Message;
use crate::state::AppState;
use crate::styles;

pub(super) fn view<'a>(state: &'a AppState, theme: &Theme) -> Element<'a, Message> {
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
        if !slide.title.is_blank() {
            let title_content = content::render_content(&slide.title);
            content_column = content_column.push(widget::text(title_content).size(32.0));
        }

        // Body with markdown rendering
        if !slide.body.is_blank() {
            let body_element: Element<'_, Message> = if let Some(md) = cached_md {
                // From the live theme, not from `Theme::Dark`. A `Settings`
                // is a value rather than a style closure, so it cannot be
                // handed the theme later — which is how this came to hardcode
                // one and why a light theme was unreachable.
                let settings = markdown::Settings::with_text_size(FONT_SIZE_BODY, theme);
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
                .height(Length::FillPortion(PORTION_BODY))
                .anchor_top(),
            );
        }

        // Speaker Notes with markdown rendering.
        //
        // `is_blank`, not the `matches!(.., Content::Text { text } if
        // text.is_empty())` this replaces: that is `false` for `Content::Empty`,
        // which is exactly what the server sends for a slide with no notes —
        // `Slide` skips the field with `Content::is_empty` — so the negation
        // passed and every such slide was drawn with an empty notes box.
        if !slide.notes.is_blank() {
            let notes_element: Element<'_, Message> = if let Some(md) = cached_md {
                let settings = markdown::Settings::with_text_size(FONT_SIZE_NOTES, theme);
                markdown::view(&md.notes_items, settings).map(Message::LinkClicked)
            } else {
                content::render_content_element(&slide.notes)
            };

            content_column = content_column.push(
                container(
                    column![
                        widget::text("Speaker Notes")
                            .size(FONT_SIZE_LARGE)
                            .style(widget::text::secondary),
                        container(
                            // Anchored to the top: notes are read from their
                            // first line down, and this was anchored to the
                            // bottom, so a long note opened on its last
                            // sentence.
                            scrollable(
                                container(notes_element)
                                    .padding(PADDING_CONTAINER)
                                    .width(Length::Fill)
                            )
                            .height(Length::Fill)
                            .anchor_top()
                        )
                        .height(Length::Fill)
                        .style(styles::preview_container())
                    ]
                    .spacing(SPACING_SMALL),
                )
                // A share of the window rather than 150 fixed pixels, which was
                // 150 on a 720p screen and 150 on a 4K one — three lines of
                // notes on the display a presenter actually uses.
                .height(Length::FillPortion(PORTION_NOTES))
                .width(Length::Fill),
            );
        }

        if let Some(preview) = super::sidebar::next_slide_preview(state) {
            content_column = content_column.push(preview);
        }

        // Top-aligned, not centred. `center_y` pushed the column into the
        // middle of the window while the body inside it was scrolling, so the
        // pane had empty space above and below content that did not fit.
        container(content_column)
            .width(Length::Fill)
            .height(Length::Fill)
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
