use std::fmt::Display;
use std::time::Duration;

use toboggan_core::{ClientId, State};

mod api;
pub use self::api::TobogganApi;

mod communication;
pub use self::communication::CommunicationService;

mod keyboard;
pub use self::keyboard::*;

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Closed,
    Reconnecting {
        attempt: usize,
        max_attempt: usize,
        delay: Duration,
    },
    Error {
        message: String,
    },
}

impl Display for ConnectionStatus {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connecting => write!(fmt, "📡 Connecting..."),
            Self::Connected => write!(fmt, "🛜 Connected"),
            Self::Closed => write!(fmt, "🚪 Closed"),
            Self::Error { message } => write!(fmt, "💥 Error: {message}"),
            Self::Reconnecting {
                attempt,
                max_attempt,
                delay,
            } => write!(
                fmt,
                "⛓️‍💥 Reconnecting in {}s {attempt}/{max_attempt}",
                delay.as_secs()
            ),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum CommunicationMessage {
    ConnectionStatusChange { status: ConnectionStatus },
    StateChange { state: State },
    TalkChange { state: State },
    Registered { client_id: ClientId },
    ClientConnected { client_id: ClientId, name: String },
    ClientDisconnected { client_id: ClientId, name: String },
    Error { error: String },
}
