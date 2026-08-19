use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{ClientId, ClientRole, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
/// What the server tells its clients.
///
/// Most notifications are broadcast to everyone — the deck is shared state, so
/// a change one client asked for is news for all of them. [`Self::Error`] and
/// [`Self::Registered`] go only to the client concerned.
#[serde(tag = "type")]
pub enum Notification {
    /// The deck moved. Broadcast to every client after every change.
    State {
        /// Where the presentation now is.
        state: State,
    },
    /// Something went wrong, for this client only.
    ///
    /// Used rather than a disconnect when an audience client sends a command it
    /// is not allowed to send: a stale tab is a mistake, not an attack.
    Error {
        /// What went wrong, in words a presenter can read.
        message: String,
    },
    /// Answer to [`crate::Command::Ping`].
    Pong,
    /// Flash the screen — someone pressed `b`.
    Blink,
    /// The deck itself was rebuilt, so clients should re-fetch it.
    ///
    /// Sent by the file watcher in live-reload mode.
    TalkChange {
        /// Where the presentation is in the new deck.
        state: State,
    },
    /// Sent to the registering client with their assigned ID and the role the
    /// server granted them.
    ///
    /// The role is told rather than asked for: a client cannot know whether it
    /// may drive the deck until the server has looked at where it connected
    /// from, so this is where a UI learns to hide its navigation controls.
    Registered {
        /// The id the server assigned to this connection.
        client_id: ClientId,
        /// What this client is allowed to do.
        #[serde(default)]
        role: ClientRole,
    },
    /// Broadcast when a client connects
    ClientConnected {
        /// The client that joined.
        client_id: ClientId,
        /// The name it registered under.
        name: String,
    },
    /// Broadcast when a client disconnects
    ClientDisconnected {
        /// The client that left.
        client_id: ClientId,
        /// The name it had registered under.
        name: String,
    },
}

impl Notification {
    /// The blink notification, as a constant.
    pub const BLINK: Self = Self::Blink;
    /// The heartbeat answer, as a constant.
    pub const PONG: Self = Self::Pong;

    /// A state broadcast.
    #[must_use]
    pub fn state(state: State) -> Self {
        Self::State { state }
    }

    #[must_use]
    /// A notice that the deck was rebuilt.
    pub fn talk_change(state: State) -> Self {
        Self::TalkChange { state }
    }

    /// An error for one client, formatted from anything printable.
    pub fn error(err: impl Debug) -> Self {
        let message = format!("{err:?}");
        Self::Error { message }
    }

    #[must_use]
    /// The answer to a successful registration.
    pub fn registered(client_id: ClientId, role: ClientRole) -> Self {
        Self::Registered { client_id, role }
    }

    #[must_use]
    /// A notice that a client joined.
    pub fn client_connected(client_id: ClientId, name: impl Into<String>) -> Self {
        Self::ClientConnected {
            client_id,
            name: name.into(),
        }
    }

    #[must_use]
    /// A notice that a client left.
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
