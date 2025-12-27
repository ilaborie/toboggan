use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use slotmap::DefaultKey;

use crate::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(DefaultKey);

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClientInfo {
    pub id: ClientId,
    pub name: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub ip_addr: IpAddr,
    pub connected_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClientsResponse {
    pub clients: Vec<ClientInfo>,
}
