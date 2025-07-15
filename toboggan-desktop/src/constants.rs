use iced::Color;

// Font sizes - consider using theme-based typography in the future
pub const FONT_SIZE_SMALL: u16 = 12;
pub const FONT_SIZE_MEDIUM: u16 = 14;
pub const FONT_SIZE_LARGE: u16 = 16;
pub const FONT_SIZE_TITLE: u16 = 18;

// Legacy colors - prefer theme.extended_palette() colors when possible
pub const COLOR_MUTED: Color = Color::from_rgb(0.6, 0.6, 0.6);

// Spacing
pub const SPACING_SMALL: u16 = 4;
pub const SPACING_MEDIUM: u16 = 8;
pub const SPACING_LARGE: u16 = 12;

// Padding values
pub const PADDING_SMALL: [u16; 2] = [2, 4];
pub const PADDING_MEDIUM: [u16; 2] = [3, 6];
pub const PADDING_CONTAINER: u16 = 6;
pub const PADDING_SLIDE_CONTENT: u16 = 20;

// Border radius
pub const BORDER_RADIUS: f32 = 4.0;
pub const BORDER_WIDTH: f32 = 1.0;

// Icon sizes
pub const ICON_SIZE_SMALL: u16 = 14;
pub const ICON_SIZE_MEDIUM: u16 = 16;

// Component dimensions
pub const SLIDE_NOTES_HEIGHT: f32 = 150.0;
pub const SLIDE_NOTES_SCROLL_HEIGHT: f32 = 130.0;
