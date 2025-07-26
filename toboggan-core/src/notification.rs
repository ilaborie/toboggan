use alloc::string::String;
use core::fmt::Debug;
use std::format;

use serde::{Deserialize, Serialize};

use crate::{State, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
