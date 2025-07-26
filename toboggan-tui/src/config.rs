#[derive(Debug, Clone)]
pub struct Config {
    pub websocket_url: String,
    pub api_url: String,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            websocket_url: "ws://localhost:8080/api/ws".to_string(),
            api_url: "http://localhost:8080".to_string(),
            max_retries: 5,
            retry_delay_ms: 1000,
        }
    }
}
