use iced::widget::{Column, column, container, row, text};
use iced::{Element, Length};
use toboggan_core::Content;

use crate::messages::Message;

pub fn render_content(content: &Content) -> Element<'_, Message> {
    match content {
        Content::Empty => container(text("")).into(),

        Content::Text { text: txt } => container(text(txt).size(24)).width(Length::Fill).into(),

        Content::Html { raw, alt } => {
            // For now, display alt text or raw HTML as text
            // TODO: Implement proper HTML rendering with a WebView
            let display_text = alt.as_ref().unwrap_or(raw);
            container(text(display_text).size(20))
                .width(Length::Fill)
                .into()
        }

        Content::IFrame { url } => {
            // For now, just show the URL
            // TODO: Implement iframe support with WebView
            container(column![text("IFrame:").size(16), text(url).size(14),].spacing(5))
                .width(Length::Fill)
                .into()
        }

        Content::Term { cwd, bootstrap } => {
            // For now, show terminal info
            // TODO: Implement terminal emulation
            let commands = bootstrap.join(" && ");
            container(
                column![
                    text("Terminal:").size(16),
                    text(format!("Working Directory: {}", cwd.display())).size(14),
                    text(format!("Commands: {commands}")).size(14),
                ]
                .spacing(5),
            )
            .width(Length::Fill)
            .into()
        }

        Content::HBox {
            columns: _,
            contents,
        } => {
            let mut row_contents = row![].spacing(10);
            for content in contents {
                row_contents = row_contents.push(render_content(content));
            }
            container(row_contents).width(Length::Fill).into()
        }

        Content::VBox { rows: _, contents } => {
            let mut column_contents = column![].spacing(10);
            for content in contents {
                column_contents = column_contents.push(render_content(content));
            }
            container(column_contents).width(Length::Fill).into()
        }
    }
}

#[allow(dead_code)]
pub fn render_content_column(contents: &[Content]) -> Column<'_, Message> {
    let mut column = column![].spacing(10);
    for content in contents {
        column = column.push(render_content(content));
    }
    column
}
