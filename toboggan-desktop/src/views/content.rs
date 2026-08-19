use iced::Element;
use toboggan_core::Content;

use crate::message::Message;

pub(super) fn render_content(content: &Content) -> String {
    content.display_text().to_owned()
}

pub(super) fn render_content_element(content: &Content) -> Element<'_, Message> {
    match content {
        Content::Empty => iced::widget::text("").size(20.0).into(),
        Content::Text { text } => {
            // Render as text with proper styling
            iced::widget::text(text).size(20.0).into()
        }
        Content::Html { .. } => iced::widget::text(content.display_text()).size(20.0).into(),
    }
}
