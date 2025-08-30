//! Common configuration patterns for all Toboggan clients
//!
//! This module provides shared configuration structures and traits
//! that can be used across all client implementations to ensure
//! consistency and reduce code duplication.

use alloc::format;
use alloc::string::{String, ToString};
#[cfg(feature = "std")]
use std::time::Duration;

use crate::ClientId;

#[cfg(feature = "std")]
pub mod duration_serde {
    //! Serde helpers for Duration with humantime parsing support

    use alloc::string::{String, ToString};
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        humantime::format_duration(*duration)
            .to_string()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        humantime::parse_duration(&s).map_err(serde::de::Error::custom)
    }
}

/// Common client configuration trait
pub trait ClientConfig {
    /// Get the client ID
    fn client_id(&self) -> &ClientId;

    /// Get the API base URL
    fn api_url(&self) -> &str;

    /// Get the WebSocket URL
    fn websocket_url(&self) -> &str;
}

/// WebSocket retry configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: usize,

    /// Initial delay between retries
    #[cfg_attr(
        feature = "std",
        serde(with = "duration_serde", default = "default_initial_retry_delay")
    )]
    #[cfg(feature = "std")]
    pub initial_retry_delay: Duration,

    /// Initial delay between retries (in milliseconds for `no_std` compatibility)
    #[cfg(not(feature = "std"))]
    pub initial_retry_delay_ms: u64,

    /// Maximum retry delay
    #[cfg_attr(
        feature = "std",
        serde(with = "duration_serde", default = "default_max_retry_delay")
    )]
    #[cfg(feature = "std")]
    pub max_retry_delay: Duration,

    /// Maximum retry delay (in milliseconds for `no_std` compatibility)
    #[cfg(not(feature = "std"))]
    pub max_retry_delay_ms: u64,

    /// Exponential backoff factor (e.g., 2.0 for doubling)
    pub backoff_factor: f32,

    /// Whether to add jitter to retry delays
    pub use_jitter: bool,
}

#[cfg(feature = "std")]
fn default_initial_retry_delay() -> Duration {
    Duration::from_secs(1)
}

#[cfg(feature = "std")]
fn default_max_retry_delay() -> Duration {
    Duration::from_secs(30)
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 10,
            #[cfg(feature = "std")]
            initial_retry_delay: Duration::from_secs(1),
            #[cfg(not(feature = "std"))]
            initial_retry_delay_ms: 1_000,
            #[cfg(feature = "std")]
            max_retry_delay: Duration::from_secs(30),
            #[cfg(not(feature = "std"))]
            max_retry_delay_ms: 30_000,
            backoff_factor: 2.0,
            use_jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration with Duration (std only)
    #[cfg(feature = "std")]
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

    /// Create a new retry configuration with milliseconds (no_std compatible)
    #[cfg(not(feature = "std"))]
    #[must_use]
    pub const fn new(
        max_retries: usize,
        initial_retry_delay_ms: u64,
        max_retry_delay_ms: u64,
        backoff_factor: f32,
        use_jitter: bool,
    ) -> Self {
        Self {
            max_retries,
            initial_retry_delay_ms,
            max_retry_delay_ms,
            backoff_factor,
            use_jitter,
        }
    }

    /// Calculate the delay for a given retry attempt (returns milliseconds)
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn calculate_delay(&self, attempt: usize) -> u64 {
        #[cfg(feature = "std")]
        let initial_ms = self.initial_retry_delay.as_millis() as u64;
        #[cfg(not(feature = "std"))]
        let initial_ms = self.initial_retry_delay_ms;

        #[cfg(feature = "std")]
        let max_ms = self.max_retry_delay.as_millis() as u64;
        #[cfg(not(feature = "std"))]
        let max_ms = self.max_retry_delay_ms;

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
            #[cfg(feature = "std")]
            {
                use std::collections::hash_map::RandomState;
                use std::hash::BuildHasher;

                let random_state = RandomState::new();
                let jitter = (random_state.hash_one(attempt) % 20) as f32 / 100.0;
                delay = (delay as f32 * (1.0 + jitter)) as u64;
            }
        }

        delay
    }

    /// Get initial retry delay as Duration (when std is available)
    #[cfg(feature = "std")]
    #[must_use]
    pub const fn initial_retry_delay(&self) -> Duration {
        self.initial_retry_delay
    }

    /// Get max retry delay as Duration (when std is available)
    #[cfg(feature = "std")]
    #[must_use]
    pub const fn max_retry_delay(&self) -> Duration {
        self.max_retry_delay
    }

    /// Get initial retry delay in milliseconds
    #[must_use]
    pub fn initial_retry_delay_ms(&self) -> u64 {
        #[cfg(feature = "std")]
        return self.initial_retry_delay.as_millis() as u64;
        #[cfg(not(feature = "std"))]
        return self.initial_retry_delay_ms;
    }

    /// Get max retry delay in milliseconds
    #[must_use]
    pub fn max_retry_delay_ms(&self) -> u64 {
        #[cfg(feature = "std")]
        return self.max_retry_delay.as_millis() as u64;
        #[cfg(not(feature = "std"))]
        return self.max_retry_delay_ms;
    }
}

/// Base configuration for all Toboggan clients
#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseClientConfig {
    /// Unique client identifier
    pub client_id: ClientId,

    /// Base URL for API requests
    pub api_url: String,

    /// WebSocket URL for real-time communication
    pub websocket_url: String,

    /// Retry configuration for connection failures
    pub retry: RetryConfig,
}

impl BaseClientConfig {
    /// Create a new base configuration
    #[must_use]
    pub fn new(api_url: impl Into<String>, websocket_url: impl Into<String>) -> Self {
        Self {
            client_id: ClientId::new(),
            api_url: api_url.into(),
            websocket_url: websocket_url.into(),
            retry: RetryConfig::default(),
        }
    }

    /// Create configuration for localhost with default ports
    #[must_use]
    pub fn localhost() -> Self {
        Self::new("http://localhost:8080", "ws://localhost:8080/api/ws")
    }

    /// Create configuration from a base URL
    #[must_use]
    pub fn from_base_url(base_url: impl AsRef<str>) -> Self {
        let base = base_url.as_ref().trim_end_matches('/');
        let ws_base = if base.starts_with("https://") {
            base.replace("https://", "wss://")
        } else if base.starts_with("http://") {
            base.replace("http://", "ws://")
        } else {
            format!("ws://{base}")
        };

        Self::new(base.to_string(), format!("{ws_base}/api/ws"))
    }

    /// Set the retry configuration
    #[must_use]
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Set the client ID
    #[must_use]
    pub fn with_client_id(mut self, client_id: ClientId) -> Self {
        self.client_id = client_id;
        self
    }
}

impl ClientConfig for BaseClientConfig {
    fn client_id(&self) -> &ClientId {
        &self.client_id
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
    #[cfg(feature = "std")]
    use std::time::Duration;

    /// Default heartbeat interval in milliseconds
    pub const HEARTBEAT_INTERVAL_MS: u64 = 30_000;

    /// Default connection timeout in milliseconds
    pub const CONNECTION_TIMEOUT_MS: u64 = 10_000;

    /// Default ping interval in milliseconds
    pub const PING_INTERVAL_MS: u64 = 25_000;

    /// Default heartbeat interval as Duration (std only)
    #[cfg(feature = "std")]
    pub const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(HEARTBEAT_INTERVAL_MS);

    /// Default connection timeout as Duration (std only)
    #[cfg(feature = "std")]
    pub const CONNECTION_TIMEOUT: Duration = Duration::from_millis(CONNECTION_TIMEOUT_MS);

    /// Default ping interval as Duration (std only)
    #[cfg(feature = "std")]
    pub const PING_INTERVAL: Duration = Duration::from_millis(PING_INTERVAL_MS);
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
        let delay_max = config.calculate_delay(100);
        let max_delay_ms = config.max_retry_delay_ms();
        assert!(delay_max <= max_delay_ms + (max_delay_ms / 5));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_humantime_serialization() {
        let config = RetryConfig {
            max_retries: 5,
            initial_retry_delay: Duration::from_secs(2),
            max_retry_delay: Duration::from_secs(60),
            backoff_factor: 1.5,
            use_jitter: false,
        };

        let serialized = serde_json::to_string(&config).expect("Should serialize");
        let deserialized: RetryConfig =
            serde_json::from_str(&serialized).expect("Should deserialize");

        assert_eq!(config.max_retries, deserialized.max_retries);
        assert_eq!(config.initial_retry_delay, deserialized.initial_retry_delay);
        assert_eq!(config.max_retry_delay, deserialized.max_retry_delay);
        assert_eq!(config.backoff_factor, deserialized.backoff_factor);
        assert_eq!(config.use_jitter, deserialized.use_jitter);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_humantime_parsing() {
        let json = r#"{
            "max_retries": 3,
            "initial_retry_delay": "1s",
            "max_retry_delay": "30s",
            "backoff_factor": 2.0,
            "use_jitter": true
        }"#;

        let config: RetryConfig =
            serde_json::from_str(json).expect("Should parse humantime durations");

        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_retry_delay, Duration::from_secs(1));
        assert_eq!(config.max_retry_delay, Duration::from_secs(30));
        assert_eq!(config.backoff_factor, 2.0);
        assert!(config.use_jitter);
    }

    #[test]
    fn test_base_config_from_base_url() {
        let config = BaseClientConfig::from_base_url("https://example.com");
        assert_eq!(config.api_url, "https://example.com");
        assert_eq!(config.websocket_url, "wss://example.com/api/ws");

        let config = BaseClientConfig::from_base_url("http://localhost:3000/");
        assert_eq!(config.api_url, "http://localhost:3000");
        assert_eq!(config.websocket_url, "ws://localhost:3000/api/ws");
    }
}
