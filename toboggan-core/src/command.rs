use serde::{Deserialize, Serialize};

use crate::SlideId;

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
    Full,
}
