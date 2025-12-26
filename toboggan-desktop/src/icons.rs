//! Simple icon rendering using Lucide font with iced 0.14
//!
//! This module provides icon widgets that work with iced 0.14 by using
//! the Lucide font directly with iced's text widget.

use iced::widget::text;
use iced::{Element, Font};
// Re-export Icon for convenience at call sites
pub use lucide_icons::Icon;

use crate::message::Message;

/// The Lucide font family for use with iced
pub const LUCIDE_FONT: Font = Font::with_name("lucide");

/// Creates an icon element with the specified icon and size
pub fn icon(i: Icon, size: f32) -> Element<'static, Message> {
    let unicode_char = i.unicode();
    text(unicode_char.to_string())
        .font(LUCIDE_FONT)
        .size(size)
        .into()
}
