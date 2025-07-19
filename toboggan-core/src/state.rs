use core::time::Duration;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::SlideId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum State {
    Paused {
        current: SlideId,
        total_duration: Duration,
    },
    Running {
        since: Timestamp,
        current: SlideId,
        total_duration: Duration,
    },
    Done {
        current: SlideId,
        total_duration: Duration,
    },
}
