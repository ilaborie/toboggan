use iced::keyboard;
use toboggan_client::CommunicationMessage;
use toboggan_core::{Command as TobogganCommand, Slide, SlidesResponse};

// All WebSocket commands are now unified under SendCommand variant
#[derive(Debug, Clone)]
pub enum Message {
    // Connection events - user actions only
    Connect,
    Disconnect,

    // Command execution - unified variant for all WebSocket commands
    SendCommand(TobogganCommand),

    // Data loading
    TalkLoaded(toboggan_core::TalkResponse),
    TalkAndSlidesLoaded(toboggan_core::TalkResponse, SlidesResponse),
    SlideLoaded(usize, Slide),
    LoadError(String),

    // WebSocket message handling
    Communication(CommunicationMessage),

    // UI events
    ToggleHelp,
    ToggleSidebar,
    ToggleFullscreen,
    KeyPressed(keyboard::Key, keyboard::Modifiers),
    WindowResized(f32, f32),

    // Tick for periodic updates
    Tick,
}
