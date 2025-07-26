use std::collections::HashMap;

use toboggan_core::{Notification, Slide, SlideId};

#[derive(Debug, Clone)]
pub enum Message {
    // WebSocket events
    WebSocketConnected,
    WebSocketDisconnected,
    WebSocketError(String),
    NotificationReceived(Notification),

    // Navigation commands
    FirstSlide,
    PreviousSlide,
    NextSlide,
    LastSlide,
    PlayPresentation,
    PausePresentation,
    #[allow(dead_code)]
    GoToSlide(SlideId),

    // UI events
    #[allow(dead_code)]
    WindowResized,
    ConnectToServer,
    #[allow(dead_code)]
    Tick,
    SlidesLoaded(HashMap<SlideId, Slide>),
    TalkLoaded(toboggan_core::Talk),
}
