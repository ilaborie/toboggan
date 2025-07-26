use crossterm::event::KeyEvent;
use toboggan_core::{Notification, Slide, SlideId};

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
    Connected,
    Disconnected,
    NotificationReceived(Notification),
    ConnectionError(String),
    SlideLoaded(SlideId, Slide),
    SlideLoadError(SlideId, String),
    Quit,
}

#[derive(Debug, Clone)]
pub enum AppAction {
    SendCommand(toboggan_core::Command),
    ToggleHelp,
    ShowError(String),
    ClearError,
    Connect,
    Disconnect,
    Quit,
}
