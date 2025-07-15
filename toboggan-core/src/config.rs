use serde::{Deserialize, Serialize};

use crate::{ClientId, Duration};

pub trait ClientConfig {
    fn client_id(&self) -> ClientId;
    fn api_url(&self) -> &str;
    fn websocket_url(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub initial_retry_delay: Duration,
    pub max_retry_delay: Duration,
    pub backoff_factor: f32,
    pub use_jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 10,
            initial_retry_delay: Duration::from_secs(1),
            max_retry_delay: Duration::from_secs(30),
            backoff_factor: 2.0,
            use_jitter: true,
        }
    }
}

impl RetryConfig {
    #[must_use]
    pub const fn new(
        max_retries: usize,
        initial_retry_delay: Duration,
        max_retry_delay: Duration,
        backoff_factor: f32,
        use_jitter: bool,
    ) -> Self {
        Self {
            max_retries,
            initial_retry_delay,
            max_retry_delay,
            backoff_factor,
            use_jitter,
        }
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn calculate_delay(&self, attempt: usize) -> u64 {
        let initial_ms = self.initial_retry_delay.as_millis() as u64;
        let max_ms = self.max_retry_delay.as_millis() as u64;

        if attempt == 0 {
            return initial_ms;
        }

        let mut delay = initial_ms as f32;
        for _ in 0..attempt {
            delay *= self.backoff_factor;
        }

        let mut delay = delay.min(max_ms as f32) as u64;

        if self.use_jitter {
            // Add up to 20% jitter
            let mut random_byte = [0u8; 1];
            let _ = getrandom::fill(&mut random_byte);
            let jitter = f32::from(random_byte[0] % 20) / 100.0;
            delay = (delay as f32 * (1.0 + jitter)) as u64;
        }

        delay
    }

    #[must_use]
    pub const fn initial_retry_delay(&self) -> Duration {
        self.initial_retry_delay
    }

    #[must_use]
    pub const fn max_retry_delay(&self) -> Duration {
        self.max_retry_delay
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseClientConfig {
    pub client_id: ClientId,
    pub api_url: String,
    pub websocket_url: String,
    pub retry: RetryConfig,
}

impl BaseClientConfig {
    #[must_use]
    pub fn new(host: &str, port: u16) -> Self {
        let api_url = format!("http://{host}:{port}");
        let websocket_url = format!("ws://{host}:{port}/api/ws");
        Self {
            client_id: ClientId::new(),
            api_url,
            websocket_url,
            retry: RetryConfig::default(),
        }
    }

    #[must_use]
    pub fn localhost() -> Self {
        Self::new("localhost", 8080)
    }

    #[must_use]
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }
}

impl ClientConfig for BaseClientConfig {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn api_url(&self) -> &str {
        &self.api_url
    }

    fn websocket_url(&self) -> &str {
        &self.websocket_url
    }
}

impl Default for BaseClientConfig {
    fn default() -> Self {
        Self::localhost()
    }
}

/// Connection status constants for consistency across clients
pub mod connection_timeouts {
    use std::time::Duration;

    pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
    pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
    pub const PING_INTERVAL: Duration = Duration::from_secs(25);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_delay_calculation() {
        let config = RetryConfig::default();

        assert_eq!(config.calculate_delay(0), 1_000);

        // With exponential backoff factor of 2.0
        let delay1 = config.calculate_delay(1);
        assert!((2_000..=2_400).contains(&delay1)); // With jitter

        // Should not exceed max delay
        let delay_max = u128::from(config.calculate_delay(100));
        let max_delay_ms = config.max_retry_delay().as_millis();
        assert!(delay_max <= max_delay_ms + (max_delay_ms / 5));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_humantime_serialization() {
        let config = RetryConfig {
            max_retries: 5,
            initial_retry_delay: Duration::from_secs(2),
            max_retry_delay: Duration::from_secs(60),
            backoff_factor: 1.5,
            use_jitter: false,
        };

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: RetryConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(config.max_retries, deserialized.max_retries);
        assert_eq!(config.initial_retry_delay, deserialized.initial_retry_delay);
        assert_eq!(config.max_retry_delay, deserialized.max_retry_delay);
        assert!((config.backoff_factor - deserialized.backoff_factor).abs() < f32::EPSILON);
        assert_eq!(config.use_jitter, deserialized.use_jitter);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_humantime_parsing() {
        let json = r#"{
            "max_retries": 3,
            "initial_retry_delay": "1s",
            "max_retry_delay": "30s",
            "backoff_factor": 2.0,
            "use_jitter": true
        }"#;

        let config: RetryConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_retry_delay, Duration::from_secs(1));
        assert_eq!(config.max_retry_delay, Duration::from_secs(30));
        assert!((config.backoff_factor - 2.0).abs() < f32::EPSILON);
        assert!(config.use_jitter);
    }
}
