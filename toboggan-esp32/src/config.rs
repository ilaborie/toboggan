/// Configuration constants for the ESP32-S3-BOX-3B application
/// Following clean code principles by centralizing magic numbers and configuration
use embedded_graphics::pixelcolor::Rgb565;

/// LED Configuration
pub mod led {
    /// LED blink intervals in milliseconds
    pub const BLINK_INTERVAL_FAST: u64 = 250;
    pub const BLINK_INTERVAL_NORMAL: u64 = 500;
}

/// Display Configuration
pub mod display {
    use super::Rgb565;

    /// Text rendering constants
    pub const LINE_HEIGHT: i32 = 16;
    pub const BUFFER_SIZE: usize = 512;

    /// Display layout constants
    pub const TITLE_LINE_START: i32 = 2;
    pub const TITLE_MAX_LINES: usize = 2;
    pub const CURRENT_SLIDE_LINE_START: i32 = 7;
    pub const CURRENT_SLIDE_MAX_LINES: usize = 5;
    pub const NEXT_SLIDE_LINE_START: i32 = 13;
    pub const NEXT_SLIDE_MAX_LINES: usize = 2;
    pub const MAX_CHARS_PER_LINE: usize = 32;

    /// Display colors for different states - using explicit RGB values
    pub const COLOR_BLACK: Rgb565 = Rgb565::new(0x00, 0x00, 0x00);
    pub const COLOR_WHITE: Rgb565 = Rgb565::new(0x1F, 0x3F, 0x1F);
    pub const COLOR_RED: Rgb565 = Rgb565::new(0x1F, 0x00, 0x00);
    pub const COLOR_YELLOW: Rgb565 = Rgb565::new(0x1F, 0x3F, 0x00);
    pub const COLOR_CYAN: Rgb565 = Rgb565::new(0x00, 0x3F, 0x1F);
    pub const COLOR_GREEN: Rgb565 = Rgb565::new(0x00, 0x3F, 0x00);

    /// Fixed orange color for display text
    pub const COLOR_ORANGE: Rgb565 = Rgb565::new(0x1F, 0x0F, 0x00); // True orange

    /// Error background color
    pub const COLOR_ERROR_BACKGROUND: Rgb565 = Rgb565::new(0x10, 0x00, 0x00);
}

/// Timing Configuration
pub mod timing {
    use std::time::Duration;

    /// Main loop delays
    pub const MAIN_LOOP_POLL_INTERVAL: Duration = Duration::from_millis(100);
}

/// Application Constants
pub mod app {
    /// Default text values
    pub const ERROR_PREFIX: &str = "Error! ";
    pub const BOOTING_TEXT: &str = "Booting...";
}

/// Network Configuration
pub mod network {
    pub const CONNECTING_TEXT_PREFIX: &str = "Connecting to Wifi ";
    pub const CONNECTING_TEXT_SUFFIX: &str = "...";
    pub const LOADING_TALK_TEXT: &str = "Loading talk...";
}
