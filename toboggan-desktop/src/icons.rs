//! Simple icon rendering using Lucide font with iced 0.14
//!
//! This module provides icon widgets that work with iced 0.14 by using
//! the Lucide font directly with iced's text widget.

use iced::widget::text;
use iced::{Element, Font};
use lucide_icons::Icon;

use crate::message::Message;

/// The Lucide font family for use with iced
pub const LUCIDE_FONT: Font = Font::with_name("lucide");

/// Creates an icon element with the specified icon and size
pub fn icon(icon: Icon, size: f32) -> Element<'static, Message> {
    let unicode_char = icon.unicode();
    text(unicode_char.to_string())
        .font(LUCIDE_FONT)
        .size(size)
        .into()
}

// Convenience functions for commonly used icons

pub fn icon_bell(size: f32) -> Element<'static, Message> {
    icon(Icon::Bell, size)
}

pub fn icon_chevron_left(size: f32) -> Element<'static, Message> {
    icon(Icon::ChevronLeft, size)
}

pub fn icon_chevron_right(size: f32) -> Element<'static, Message> {
    icon(Icon::ChevronRight, size)
}

pub fn icon_loader(size: f32) -> Element<'static, Message> {
    icon(Icon::Loader, size)
}

pub fn icon_pause(size: f32) -> Element<'static, Message> {
    icon(Icon::Pause, size)
}

pub fn icon_play(size: f32) -> Element<'static, Message> {
    icon(Icon::Play, size)
}

pub fn icon_refresh_cw(size: f32) -> Element<'static, Message> {
    icon(Icon::RefreshCw, size)
}

pub fn icon_skip_back(size: f32) -> Element<'static, Message> {
    icon(Icon::SkipBack, size)
}

pub fn icon_skip_forward(size: f32) -> Element<'static, Message> {
    icon(Icon::SkipForward, size)
}

pub fn icon_wifi(size: f32) -> Element<'static, Message> {
    icon(Icon::Wifi, size)
}

pub fn icon_wifi_off(size: f32) -> Element<'static, Message> {
    icon(Icon::WifiOff, size)
}

pub fn icon_x(size: f32) -> Element<'static, Message> {
    icon(Icon::X, size)
}
