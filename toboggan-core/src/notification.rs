use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{ClientId, ClientRole, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type")]
pub enum Notification {
    State {
        state: State,
    },
    Error {
        message: String,
    },
    Pong,
    Blink,
    TalkChange {
        state: State,
    },
    /// Sent to the registering client with their assigned ID and the role the
    /// server granted them.
    ///
    /// The role is told rather than asked for: a client cannot know whether it
    /// may drive the deck until the server has looked at where it connected
    /// from, so this is where a UI learns to hide its navigation controls.
    Registered {
        client_id: ClientId,
        #[serde(default)]
        role: ClientRole,
    },
    /// Broadcast when a client connects
    ClientConnected {
        client_id: ClientId,
        name: String,
    },
    /// Broadcast when a client disconnects
    ClientDisconnected {
        client_id: ClientId,
        name: String,
    },
}

impl Notification {
    pub const BLINK: Self = Self::Blink;
    pub const PONG: Self = Self::Pong;

    #[must_use]
    pub fn state(state: State) -> Self {
        Self::State { state }
    }

    #[must_use]
    pub fn talk_change(state: State) -> Self {
        Self::TalkChange { state }
    }

    pub fn error(err: impl Debug) -> Self {
        let message = format!("{err:?}");
        Self::Error { message }
    }

    #[must_use]
    pub fn registered(client_id: ClientId, role: ClientRole) -> Self {
        Self::Registered { client_id, role }
    }

    #[must_use]
    pub fn client_connected(client_id: ClientId, name: impl Into<String>) -> Self {
        Self::ClientConnected {
            client_id,
            name: name.into(),
        }
    }

    #[must_use]
    pub fn client_disconnected(client_id: ClientId, name: impl Into<String>) -> Self {
        Self::ClientDisconnected {
            client_id,
            name: name.into(),
        }
    }
}

impl From<State> for Notification {
    fn from(state: State) -> Self {
        Self::State { state }
    }
}
