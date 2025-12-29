use iced::Element;
use toboggan_core::Content;

use crate::message::Message;

pub fn render_content(content: &Content) -> String {
    match content {
        Content::Empty => String::new(),
        Content::Text { text } => text.clone(),
        Content::Html { raw, alt, .. } => alt.as_ref().unwrap_or(raw).clone(),
    }
}

pub fn render_content_element(content: &Content) -> Element<'_, Message> {
    match content {
        Content::Empty => iced::widget::text("").size(20.0).into(),
        Content::Text { text } => {
            // Render as text with proper styling
            iced::widget::text(text).size(20.0).into()
        }
        Content::Html { raw, alt, .. } => {
            // For HTML content, use alt text or raw
            let source = alt.as_ref().unwrap_or(raw);
            iced::widget::text(source).size(20.0).into()
        }
    }
}
