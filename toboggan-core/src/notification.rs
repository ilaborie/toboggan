use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type")]
pub enum Notification {
    State { state: State },
    Error { message: String },
    Pong,
    Blink,
    TalkChange { state: State },
}

impl Notification {
    pub const BLINK: Self = Self::Blink;
    pub const PONG: Self = Self::Pong;

    #[must_use]
    pub fn state(state: State) -> Self {
        Self::State { state }
    }

    #[must_use]
    pub fn talk_change(state: State) -> Self {
        Self::TalkChange { state }
    }

    pub fn error(err: impl Debug) -> Self {
        let message = format!("{err:?}");
        Self::Error { message }
    }
}

impl From<State> for Notification {
    fn from(state: State) -> Self {
        Self::State { state }
    }
}
