use alloc::string::String;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::State;

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
