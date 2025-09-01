#[cfg(feature = "openapi")]
use alloc::{string::String, vec::Vec};

#[cfg(all(not(feature = "std"), feature = "getrandom"))]
use getrandom;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClientId(Uuid);

impl ClientId {
    #[cfg(any(feature = "std", feature = "getrandom"))]
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "std")]
        {
            // Use v4 UUID (random) when std is available
            Self(Uuid::new_v4())
        }
        #[cfg(not(feature = "std"))]
        {
            #[cfg(feature = "getrandom")]
            {
                // In no_std environments, generate a v4 UUID using provided random bytes
                // This requires getrandom to be available (with js feature for WASM)
                let mut bytes = [0u8; 16];
                getrandom::getrandom(&mut bytes).expect("Failed to generate random bytes for UUID");
                Self(Uuid::from_bytes(bytes))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum Command {
    // General
    Register {
        client: ClientId,
        #[serde(default)]
        renderer: Renderer,
    },
    Unregister {
        client: ClientId,
    },
    // Move fast
    First,
    Last,
    GoTo {
        slide: usize,
    },
    // Navigation
    Next,
    Previous,
    // Pause/Resume
    Pause,
    Resume,
    // Effects
    Blink,
    // Ping
    Ping,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Renderer {
    #[default]
    Title,
    Html,
    Raw,
}
