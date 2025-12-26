use iced::Color;

// Font sizes - iced 0.14 requires f32 for Pixels
pub const FONT_SIZE_SMALL: f32 = 12.0;
pub const FONT_SIZE_MEDIUM: f32 = 14.0;
pub const FONT_SIZE_LARGE: f32 = 16.0;
pub const FONT_SIZE_TITLE: f32 = 18.0;

// Legacy colors - prefer theme.extended_palette() colors when possible
pub const COLOR_MUTED: Color = Color::from_rgb(0.6, 0.6, 0.6);

// Spacing - iced 0.14 requires f32 for Pixels
pub const SPACING_SMALL: f32 = 4.0;
pub const SPACING_MEDIUM: f32 = 8.0;
pub const SPACING_LARGE: f32 = 12.0;

// Padding values
pub const PADDING_SMALL: iced::Padding = iced::Padding {
    top: 2.0,
    right: 4.0,
    bottom: 2.0,
    left: 4.0,
};
pub const PADDING_MEDIUM: iced::Padding = iced::Padding {
    top: 3.0,
    right: 6.0,
    bottom: 3.0,
    left: 6.0,
};
pub const PADDING_CONTAINER: f32 = 6.0;
pub const PADDING_SLIDE_CONTENT: f32 = 20.0;

// Border radius
pub const BORDER_RADIUS: f32 = 4.0;
pub const BORDER_WIDTH: f32 = 1.0;

// Icon sizes
pub const ICON_SIZE_SMALL: f32 = 14.0;
pub const ICON_SIZE_MEDIUM: f32 = 16.0;

// Component dimensions
pub const SLIDE_NOTES_HEIGHT: f32 = 150.0;
pub const SLIDE_NOTES_SCROLL_HEIGHT: f32 = 130.0;
