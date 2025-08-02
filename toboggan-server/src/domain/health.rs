use std::time::Duration;

use serde::Serialize;

use toboggan_core::Timestamp;

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: HealthResponseStatus,
    pub started_at: Timestamp,
    pub elapsed: Duration,
    pub talk: String,
    pub active_clients: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HealthResponseStatus {
    Ok,
    Oops,
}
