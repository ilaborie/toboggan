use std::time::Duration;

use toboggan_core::ClientId;

#[derive(Debug, Clone)]
pub struct TobogganConfig {
    pub client_id: ClientId,
    pub api_url: String,
    pub websocket: TobogganWebsocketConfig,
}

impl Default for TobogganConfig {
    fn default() -> Self {
        Self {
            client_id: ClientId::new(),
            api_url: String::from("http://localhost:8080"),
            websocket: TobogganWebsocketConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TobogganWebsocketConfig {
    pub websocket_url: String,
    pub max_retries: usize,
    pub retry_delay: Duration,
    pub max_retry_delay: Duration,
}

impl Default for TobogganWebsocketConfig {
    fn default() -> Self {
        Self {
            websocket_url: String::from("ws://localhost:8080/api/ws"),
            max_retries: 10,
            retry_delay: Duration::from_secs(1),
            max_retry_delay: Duration::from_secs(30),
        }
    }
}
