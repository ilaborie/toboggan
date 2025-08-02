use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use toboggan_core::{Duration, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: HealthResponseStatus,
    pub started_at: Timestamp,
    pub elapsed: Duration,
    pub talk: String,
    pub active_clients: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum HealthResponseStatus {
    Ok,
    Oops,
}
