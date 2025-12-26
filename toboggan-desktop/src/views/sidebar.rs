use iced::widget::{self, button, column, container, scrollable};
use iced::{Element, Length, Padding, Theme};
use toboggan_core::{Command, Content, SlideId};

use super::content;
use crate::constants::{PADDING_CONTAINER, SPACING_MEDIUM, SPACING_SMALL};
use crate::message::Message;
use crate::state::AppState;
use crate::styles;
use crate::widgets::{create_body_text, create_title_text};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let mut sidebar_content = column![create_title_text("Slides")]
        .spacing(SPACING_MEDIUM)
        .padding(PADDING_CONTAINER);

    if state.talk.is_some() && !state.slides.is_empty() {
        let mut slides_list = column![].spacing(SPACING_SMALL);

        for (index, slide) in state.slides.iter().enumerate() {
            let slide_id = SlideId::new(index);
            let is_current = state.current_slide == Some(slide_id);

            let slide_text = if matches!(&slide.title, Content::Text { text } if text.is_empty()) {
                format!("{}. Slide {index}", index + 1)
            } else {
                format!("{}. {}", index + 1, content::render_content(&slide.title))
            };

            let slide_button = widget::button(widget::text(slide_text))
                .on_press(Message::SendCommand(Command::GoTo { slide: slide_id }))
                .padding(Padding::new(4.0).right(8.0).left(8.0))
                .style(if is_current {
                    |theme: &Theme, status| button::primary(theme, status)
                } else {
                    |theme: &Theme, status| button::secondary(theme, status)
                });

            slides_list = slides_list.push(slide_button);
        }

        // Slide list with anchor_top for stable scroll position
        sidebar_content =
            sidebar_content.push(scrollable(slides_list).height(Length::Fill).anchor_top());
    }

    // Next slide preview
    if let Some(next_slide) = state.next_slide() {
        sidebar_content = sidebar_content.push(
            container(
                column![
                    create_body_text("Next Slide"),
                    container(if matches!(&next_slide.title, toboggan_core::Content::Text { text } if text.is_empty()) {
                        widget::text("No title").size(12.0)
                    } else {
                        let title_content = content::render_content(&next_slide.title);
                        widget::text(title_content).size(12.0)
                    })
                    .padding(SPACING_SMALL)
                    .style(styles::preview_container())
                ]
                .spacing(SPACING_SMALL),
            )
            .padding(PADDING_CONTAINER),
        );
    }

    container(sidebar_content)
        .width(Length::Fixed(250.0))
        .height(Length::Fill)
        .style(styles::card_container())
        .into()
}
