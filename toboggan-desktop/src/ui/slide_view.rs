use iced::widget::{column, container};
use iced::{Background, Color, Element, Length, Theme};
use toboggan_core::{Slide, SlideKind};

use crate::messages::Message;
use crate::ui::content_renderer::render_content;

pub fn render_slide(slide: &Slide) -> Element<'_, Message> {
    let _title_size = match slide.kind {
        SlideKind::Cover => 48,
        SlideKind::Part => 36,
        SlideKind::Standard => 28,
    };

    let mut content = column![].spacing(20);

    // Title
    if !matches!(&slide.title, toboggan_core::Content::Empty) {
        content = content.push(
            container(render_content(&slide.title))
                .width(Length::Fill)
                .center_x(),
        );
    }

    // Body
    if !matches!(&slide.body, toboggan_core::Content::Empty) {
        content = content.push(
            container(render_content(&slide.body))
                .width(Length::Fill)
                .padding(20),
        );
    }

    // Apply slide-specific styling
    let styled_content = match slide.kind {
        SlideKind::Cover | SlideKind::Part => container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y(),
        SlideKind::Standard => container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(40),
    };

    styled_content.into()
}

#[allow(dead_code)]
pub fn slide_background(kind: SlideKind, _theme: &Theme) -> Background {
    match kind {
        SlideKind::Cover => Background::Color(Color::from_rgb(0.1, 0.1, 0.2)),
        SlideKind::Part => Background::Color(Color::from_rgb(0.15, 0.15, 0.25)),
        SlideKind::Standard => Background::Color(Color::from_rgb(0.05, 0.05, 0.1)),
    }
}
