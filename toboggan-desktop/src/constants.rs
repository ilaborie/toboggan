use iced::Color;

// Font sizes - iced 0.14 requires f32 for Pixels
pub(crate) const FONT_SIZE_SMALL: f32 = 12.0;
pub(crate) const FONT_SIZE_MEDIUM: f32 = 14.0;
pub(crate) const FONT_SIZE_LARGE: f32 = 16.0;
pub(crate) const FONT_SIZE_TITLE: f32 = 18.0;

// Legacy colors - prefer theme.extended_palette() colors when possible
pub(crate) const COLOR_MUTED: Color = Color::from_rgb(0.6, 0.6, 0.6);

// Spacing - iced 0.14 requires f32 for Pixels
pub(crate) const SPACING_SMALL: f32 = 4.0;
pub(crate) const SPACING_MEDIUM: f32 = 8.0;
pub(crate) const SPACING_LARGE: f32 = 12.0;

// Padding values
pub(crate) const PADDING_SMALL: iced::Padding = iced::Padding {
    top: 2.0,
    right: 4.0,
    bottom: 2.0,
    left: 4.0,
};
pub(crate) const PADDING_MEDIUM: iced::Padding = iced::Padding {
    top: 3.0,
    right: 6.0,
    bottom: 3.0,
    left: 6.0,
};
pub(crate) const PADDING_CONTAINER: f32 = 6.0;
pub(crate) const PADDING_SLIDE_CONTENT: f32 = 20.0;

// Border radius
pub(crate) const BORDER_RADIUS: f32 = 4.0;
pub(crate) const BORDER_WIDTH: f32 = 1.0;

// Icon sizes
pub(crate) const ICON_SIZE_SMALL: f32 = 14.0;
pub(crate) const ICON_SIZE_MEDIUM: f32 = 16.0;

// Layout shares rather than pixel heights. The notes box was 150px tall on a
// 720p window and on a 4K one, which meant three lines of notes on the display a
// presenter actually uses.
pub(crate) const PORTION_BODY: u16 = 3;
pub(crate) const PORTION_NOTES: u16 = 2;

/// Base size for the rendered slide body.
///
/// This pane is a reading surface for the presenter, not a mock-up of the
/// projector — it cannot be one, because the client is handed every reveal at
/// once — so it is sized to be read at a laptop's distance rather than a room's.
pub(crate) const FONT_SIZE_BODY: f32 = 18.0;

/// Base size for the speaker notes: what the speaker actually reads mid-sentence.
pub(crate) const FONT_SIZE_NOTES: f32 = 16.0;
