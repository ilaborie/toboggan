use std::fmt::Display;
use std::time::Duration;

use toboggan_core::{ClientId, ClientRole, State};

mod api;
pub(crate) use self::api::TobogganApi;

mod communication;
pub(crate) use self::communication::CommunicationService;

mod keyboard;
pub(crate) use self::keyboard::*;

#[derive(Debug, Clone)]
pub(crate) enum ConnectionStatus {
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
pub(crate) enum CommunicationMessage {
    ConnectionStatusChange {
        status: ConnectionStatus,
    },
    StateChange {
        state: State,
    },
    TalkChange {
        state: State,
    },
    Registered {
        client_id: ClientId,
        role: ClientRole,
    },
    ClientConnected {
        client_id: ClientId,
        name: String,
    },
    ClientDisconnected {
        client_id: ClientId,
        name: String,
    },
    Error {
        error: String,
    },
}
