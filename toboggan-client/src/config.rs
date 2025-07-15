use std::time::Duration;

use toboggan_core::{BaseClientConfig, ClientConfig, ClientId, RetryConfig};

#[derive(Debug, Clone, Default)]
pub struct TobogganConfig {
    base: BaseClientConfig,
}

impl TobogganConfig {
    #[must_use]
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            base: BaseClientConfig::new(host, port),
        }
    }

    #[must_use]
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.base = self.base.with_retry(retry);
        self
    }

    /// Get the WebSocket configuration for compatibility
    #[must_use]
    pub fn websocket(&self) -> TobogganWebsocketConfig {
        TobogganWebsocketConfig::from(&self.base)
    }
}

impl ClientConfig for TobogganConfig {
    fn client_id(&self) -> ClientId {
        self.base.client_id()
    }

    fn api_url(&self) -> &str {
        self.base.api_url()
    }

    fn websocket_url(&self) -> &str {
        self.base.websocket_url()
    }
}

impl From<BaseClientConfig> for TobogganConfig {
    fn from(base: BaseClientConfig) -> Self {
        Self { base }
    }
}

/// WebSocket configuration (kept for backward compatibility)
#[derive(Debug, Clone)]
pub struct TobogganWebsocketConfig {
    pub websocket_url: String,
    pub max_retries: usize,
    pub retry_delay: Duration,
    pub max_retry_delay: Duration,
}

impl From<&BaseClientConfig> for TobogganWebsocketConfig {
    fn from(config: &BaseClientConfig) -> Self {
        Self {
            websocket_url: config.websocket_url.clone(),
            max_retries: config.retry.max_retries,
            retry_delay: config.retry.initial_retry_delay().into(),
            max_retry_delay: config.retry.max_retry_delay().into(),
        }
    }
}

impl Default for TobogganWebsocketConfig {
    fn default() -> Self {
        let base = BaseClientConfig::default();
        Self::from(&base)
    }
}
