use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use slotmap::DefaultKey;

use crate::Timestamp;

/// A connected client, as the server knows it.
///
/// Assigned at registration and handed back in [`crate::Notification::Registered`].
/// Opaque on purpose: it is a slot-map key, so an id is only meaningful to the
/// server that issued it and only for as long as that connection lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(DefaultKey);

/// Which side of the projector a connection is on.
///
/// Navigation is shared state — every presenter drives the same deck — so this
/// is not about whose deck it is. It is about the two things a spectator must
/// not be able to do: move the presentation, and open a shell on the machine
/// hosting it.
///
/// [`Self::Audience`] is the default deliberately. A role that arrives unset,
/// from an older client or a hand-written frame, is the one that can do the
/// least.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ClientRole {
    /// Drives the deck, and may open the embedded terminals.
    Presenter,
    /// Follows along. Read-only.
    #[default]
    Audience,
}

impl ClientRole {
    /// Whether this role may send commands and open terminals.
    #[must_use]
    pub const fn is_presenter(self) -> bool {
        matches!(self, Self::Presenter)
    }
}

#[cfg(feature = "openapi")]
mod client_openapi {
    use std::borrow::Cow;

    use utoipa::openapi::schema::{Schema, Type};
    use utoipa::openapi::{ObjectBuilder, RefOr};
    use utoipa::{PartialSchema, ToSchema};

    use super::ClientId;

    impl ToSchema for ClientId {
        fn name() -> Cow<'static, str> {
            Cow::Borrowed("ClientId")
        }
    }

    impl PartialSchema for ClientId {
        fn schema() -> RefOr<Schema> {
            ObjectBuilder::new()
                .schema_type(Type::Object)
                .description(Some("Client identifier (server-assigned)"))
                .into()
        }
    }
}

impl ClientId {
    /// Creates a `ClientId` from a `SlotMap` `DefaultKey`
    #[must_use]
    pub fn from_key(key: DefaultKey) -> Self {
        Self(key)
    }

    /// Returns the underlying `SlotMap` key
    #[must_use]
    pub fn key(self) -> DefaultKey {
        self.0
    }
}

/// A snapshot of one connected client, as reported by `GET /api/clients`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClientInfo {
    /// Server-assigned identifier for this connection.
    pub id: ClientId,
    /// The name the client gave when it registered — `"tui"`, `"iPhone"`, …
    pub name: String,
    /// Where the connection came from, which is also what decided its role.
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub ip_addr: IpAddr,
    /// When the client registered.
    pub connected_at: Timestamp,
    /// What the server granted this client at registration.
    #[serde(default)]
    pub role: ClientRole,
}

/// The body of `GET /api/clients`: who is currently connected.
///
/// A presenter-only endpoint — the room does not get to enumerate the room.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClientsResponse {
    /// Every client currently registered with the server.
    pub clients: Vec<ClientInfo>,
}
