use iced::Element;
use iced::widget::column;
use toboggan_core::Content;

use crate::constants::SPACING_MEDIUM;
use crate::message::Message;

pub fn render_content(content: &Content) -> String {
    match content {
        Content::Empty => String::new(),
        Content::Text { text } => text.clone(),
        Content::Html { raw, alt, .. } => alt.as_ref().unwrap_or(raw).clone(),
        Content::Grid { cells, .. } => cells
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
        Content::Html { raw, alt, .. } => iced::widget::text(alt.as_ref().unwrap_or(raw))
            .size(20)
            .into(),
        Content::Grid { cells, .. } => {
            let mut col_content = column![].spacing(SPACING_MEDIUM);
            for content_item in cells {
                col_content = col_content.push(render_content_element(content_item));
            }
            col_content.into()
        }
    }
}
