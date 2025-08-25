use iced::Element;
use iced::widget::{column, container};
use toboggan_core::Content;

use crate::constants::{
    COLOR_IFRAME, COLOR_TERMINAL, FONT_SIZE_LARGE, PADDING_CONTAINER, SPACING_MEDIUM,
};
use crate::message::Message;
use crate::styles;

pub fn render_content(content: &Content) -> String {
    match content {
        Content::Empty => String::new(),
        Content::Text { text } => text.clone(),
        Content::Html { raw, alt } => alt.as_ref().unwrap_or(raw).clone(),
        Content::IFrame { url } => format!("[IFrame: {url}]"),
        Content::Term { cwd } => format!("[Terminal: {cwd}]"),
        Content::Grid { contents, .. } => contents
            .iter()
            .map(render_content)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

pub fn render_content_element(content: &Content) -> Element<'_, Message> {
    match content {
        Content::Empty => iced::widget::text("").size(20).into(),
        Content::Text { text } => iced::widget::text(text).size(20).into(),
        Content::Html { raw, alt } => iced::widget::text(alt.as_ref().unwrap_or(raw))
            .size(20)
            .into(),
        Content::IFrame { url } => {
            let iframe_text = format!("IFrame: {url}");
            container(
                iced::widget::text(iframe_text)
                    .size(FONT_SIZE_LARGE)
                    .color(COLOR_IFRAME),
            )
            .padding(PADDING_CONTAINER)
            .style(styles::iframe_container())
            .into()
        }
        Content::Term { cwd } => {
            let term_text = format!("Terminal: {cwd}");
            container(
                iced::widget::text(term_text)
                    .size(FONT_SIZE_LARGE)
                    .color(COLOR_TERMINAL),
            )
            .padding(PADDING_CONTAINER)
            .style(styles::terminal_container())
            .into()
        }
        Content::Grid { contents, .. } => {
            let mut col_content = column![].spacing(SPACING_MEDIUM);
            for content_item in contents {
                col_content = col_content.push(render_content_element(content_item));
            }
            col_content.into()
        }
    }
}
