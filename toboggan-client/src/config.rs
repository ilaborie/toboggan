use std::time::Duration;

use toboggan_core::{BaseClientConfig, ClientConfig, RetryConfig};

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

    /// Offers a presenter token, for a client connecting to a server running on
    /// another machine. See [`BaseClientConfig::with_presenter_token`].
    #[must_use]
    pub fn with_presenter_token(mut self, token: Option<&str>) -> Self {
        self.base = self.base.with_presenter_token(token);
        self
    }

    /// Get the WebSocket configuration for compatibility
    #[must_use]
    pub fn websocket(&self) -> TobogganWebsocketConfig {
        TobogganWebsocketConfig::from(&self.base)
    }
}

impl ClientConfig for TobogganConfig {
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
    /// Offered in every `Register`, including the ones sent after a reconnect —
    /// a client that dropped mid-talk has to come back as the same role it left
    /// with, or the presenter's remote goes quiet after a network blip.
    pub presenter_token: Option<String>,
}

impl From<&BaseClientConfig> for TobogganWebsocketConfig {
    fn from(config: &BaseClientConfig) -> Self {
        Self {
            websocket_url: config.websocket_url.clone(),
            max_retries: config.retry.max_retries,
            retry_delay: config.retry.initial_retry_delay().into(),
            max_retry_delay: config.retry.max_retry_delay().into(),
            presenter_token: config.presenter_token.clone(),
        }
    }
}

impl Default for TobogganWebsocketConfig {
    fn default() -> Self {
        let base = BaseClientConfig::default();
        Self::from(&base)
    }
}
