use iced::widget::{Button, Row, Text, button, row, text};
use iced::{Alignment, Element};

use crate::constants::{
    COLOR_MUTED, FONT_SIZE_MEDIUM, FONT_SIZE_SMALL, FONT_SIZE_TITLE, PADDING_MEDIUM, PADDING_SMALL,
    SPACING_MEDIUM, SPACING_SMALL,
};
use crate::message::Message;

// Helper function to create text with consistent styling
pub fn create_text(content: &str, size: u16) -> Text<'_> {
    text(content).size(size)
}

pub fn create_muted_text(content: &str) -> Text<'_> {
    text(content).size(FONT_SIZE_SMALL).color(COLOR_MUTED)
}

// Theme-aware text functions
pub fn create_title_text(content: &str) -> Text<'_> {
    text(content).size(FONT_SIZE_TITLE)
}

pub fn create_body_text(content: &str) -> Text<'_> {
    text(content).size(FONT_SIZE_MEDIUM)
}

// Helper function to create icon-text buttons
pub fn create_icon_button<'a>(
    icon: Element<'a, Message>,
    label: &'a str,
    message: Message,
) -> Button<'a, Message> {
    button(
        row![icon, text(label).size(FONT_SIZE_MEDIUM)]
            .spacing(SPACING_SMALL)
            .align_y(Alignment::Center),
    )
    .on_press(message)
    .padding(PADDING_MEDIUM)
}

pub fn create_simple_button(label: &str, message: Message) -> Button<'_, Message> {
    button(text(label).size(FONT_SIZE_MEDIUM))
        .on_press(message)
        .padding(PADDING_SMALL)
}

// Helper function to create status rows with icon and text
pub fn create_status_row<'a>(icon: Element<'a, Message>, status_text: &'a str) -> Row<'a, Message> {
    row![icon, text(status_text).size(FONT_SIZE_MEDIUM)]
        .spacing(SPACING_MEDIUM)
        .align_y(Alignment::Center)
}

pub fn create_status_row_with_button<'a>(
    icon: Element<'a, Message>,
    status_text: &'a str,
    button_elem: Element<'a, Message>,
) -> Row<'a, Message> {
    row![icon, text(status_text).size(FONT_SIZE_MEDIUM), button_elem]
        .spacing(SPACING_MEDIUM)
        .align_y(Alignment::Center)
}

// Navigation button helper
pub fn create_nav_button<'a>(
    icon: Element<'a, Message>,
    label: &'a str,
    message: Message,
    position: NavButtonPosition,
) -> Button<'a, Message> {
    let content = match position {
        NavButtonPosition::Leading => row![icon, text(label).size(FONT_SIZE_MEDIUM)],
        NavButtonPosition::Trailing => row![text(label).size(FONT_SIZE_MEDIUM), icon],
    };

    button(content.spacing(SPACING_SMALL).align_y(Alignment::Center))
        .on_press(message)
        .padding(PADDING_MEDIUM)
}

#[derive(Copy, Clone)]
pub enum NavButtonPosition {
    Leading,
    Trailing,
}
