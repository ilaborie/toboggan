use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::SlideId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ClientId(Uuid);

impl ClientId {
    #[cfg(any(feature = "std", feature = "js"))]
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "std")]
        {
            Self(Uuid::new_v4())
        }
        #[cfg(all(not(feature = "std"), feature = "js"))]
        {
            let mut bytes = [0u8; 16];
            getrandom::getrandom(&mut bytes).expect("Failed to generate random bytes for UUID");
            Self(Uuid::from_bytes(bytes))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "command")]
pub enum Command {
    Register { client: ClientId },
    Unregister { client: ClientId },
    Ping,
    // Navigation
    First,
    Last,
    GoTo { slide: SlideId },
    Next,
    Previous,
    // Step navigation
    NextStep,
    PreviousStep,
    // Effect
    Blink,
}
