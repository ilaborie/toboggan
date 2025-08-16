use alloc::format;
use alloc::string::String;
#[cfg(feature = "openapi")]
use alloc::vec::Vec;
use core::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{State, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type")]
pub enum Notification {
    State {
        timestamp: Timestamp,
        state: State,
    },
    Error {
        timestamp: Timestamp,
        message: String,
    },
    Pong {
        timestamp: Timestamp,
    },
    Blink,
}

impl Notification {
    #[must_use]
    pub fn state(state: State) -> Self {
        let timestamp = Timestamp::now();
        Self::State { timestamp, state }
    }

    pub fn error(err: impl Debug) -> Self {
        let message = format!("{err:?}");
        let timestamp = Timestamp::now();
        Self::Error { timestamp, message }
    }

    #[must_use]
    pub fn pong() -> Self {
        let timestamp = Timestamp::now();
        Self::Pong { timestamp }
    }

    #[must_use]
    pub fn blink() -> Self {
        Self::Blink
    }
}
