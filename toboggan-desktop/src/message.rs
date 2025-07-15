use iced::keyboard::{Key, Modifiers};
use toboggan_client::CommunicationMessage;
use toboggan_core::{Command as TobogganCommand, Slide, SlidesResponse, State, TalkResponse};

// All WebSocket commands are now unified under SendCommand variant
#[derive(Debug, Clone)]
pub enum Message {
    // Connection events - user actions only
    Connect,
    Disconnect,

    // Command execution - unified variant for all WebSocket commands
    SendCommand(TobogganCommand),

    // Data loading
    TalkLoaded(TalkResponse),
    TalkAndSlidesLoaded(TalkResponse, SlidesResponse),
    TalkChangeComplete(TalkResponse, SlidesResponse, State),
    SlideLoaded(usize, Slide),
    LoadError(String),

    // WebSocket message handling
    Communication(CommunicationMessage),

    // UI events
    ToggleHelp,
    ToggleSidebar,
    ToggleFullscreen,
    KeyPressed(Key, Modifiers),
    WindowResized(f32, f32),

    // Tick for periodic updates
    Tick,
}
