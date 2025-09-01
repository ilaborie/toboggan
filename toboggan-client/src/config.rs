use std::time::Duration;

use toboggan_core::{BaseClientConfig, ClientConfig, ClientId, RetryConfig};

/// Toboggan client configuration
#[derive(Debug, Clone, Default)]
pub struct TobogganConfig {
    base: BaseClientConfig,
}

impl TobogganConfig {
    /// Create a new configuration with custom URLs
    pub fn new(api_url: impl Into<String>, websocket_url: impl Into<String>) -> Self {
        Self {
            base: BaseClientConfig::new(api_url, websocket_url),
        }
    }

    /// Create configuration from a base URL
    pub fn from_base_url(base_url: impl AsRef<str>) -> Self {
        Self {
            base: BaseClientConfig::from_base_url(base_url),
        }
    }

    /// Set the retry configuration
    #[must_use]
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.base = self.base.with_retry(retry);
        self
    }

    /// Set the client ID
    #[must_use]
    pub fn with_client_id(mut self, client_id: ClientId) -> Self {
        self.base = self.base.with_client_id(client_id);
        self
    }

    /// Get the WebSocket configuration for compatibility
    #[must_use]
    pub fn websocket(&self) -> TobogganWebsocketConfig {
        TobogganWebsocketConfig::from(&self.base)
    }
}

impl ClientConfig for TobogganConfig {
    fn client_id(&self) -> &ClientId {
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
            retry_delay: Duration::from_millis(config.retry.initial_retry_delay_ms()),
            max_retry_delay: Duration::from_millis(config.retry.max_retry_delay_ms()),
        }
    }
}

impl Default for TobogganWebsocketConfig {
    fn default() -> Self {
        let base = BaseClientConfig::default();
        Self::from(&base)
    }
}
