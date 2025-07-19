use alloc::string::String;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(all(not(feature = "std"), feature = "getrandom"))]
use getrandom;

use crate::SlideId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClientId(Uuid);

impl ClientId {
    #[allow(clippy::new_without_default)]
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
            #[cfg(not(feature = "getrandom"))]
            {
                // Fallback to nil UUID when no randomness is available
                Self(Uuid::nil())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum Command {
    // General
    Register { client: String, renderer: Renderer },
    Unregister { client: String },
    // Move fast
    First,
    Last,
    GoTo(SlideId),
    // Navigation
    Next,
    Previous,
    // Pause/Resume
    Pause,
    Resume,
    // Ping
    Ping,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Renderer {
    Title,
    Thumbnail,
    Html,
    Raw,
}
