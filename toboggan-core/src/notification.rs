use serde::{Deserialize, Serialize};

use crate::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Notification {
    State(State),
    Error { message: String },
    Pong,
}
