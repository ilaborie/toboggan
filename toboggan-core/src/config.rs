use core::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

use crate::{Duration, Secret};

/// Advances when the platform RNG is unavailable, so successive retries still
/// differ from one another.
static JITTER_FALLBACK: AtomicU8 = AtomicU8::new(0);

/// A jitter fraction in `[0.0, 0.2)`, applied to the backoff delay.
///
/// Jitter exists so that a room full of clients that dropped at the same moment
/// do not all reconnect at the same moment. The RNG failure used to be
/// discarded with `let _`, which left the byte at its initial zero — no jitter
/// at all, and the schedule back to the synchronised one this is meant to break
/// up, with nothing to say it had happened.
///
/// The fallback is not random and does not decorrelate two clients, but it does
/// advance on every call, so at least a single client's retries spread out. The
/// stride is coprime with 20 so it walks the whole range rather than a few
/// values of it.
#[allow(clippy::cast_precision_loss)]
fn jitter_fraction() -> f32 {
    let mut random_byte = [0u8; 1];
    let value = match getrandom::fill(&mut random_byte) {
        Ok(()) => random_byte[0],
        Err(_) => JITTER_FALLBACK.fetch_add(7, Ordering::Relaxed),
    };
    f32::from(value % 20) / 100.0
}

/// The two addresses every client needs: where to fetch the deck, and where to
/// listen for changes to it.
pub trait ClientConfig {
    /// Base URL of the REST API, e.g. `http://localhost:8080`.
    fn api_url(&self) -> &str;
    /// URL of the synchronisation socket, e.g. `ws://localhost:8080/api/ws`.
    fn websocket_url(&self) -> &str;
}

/// How a client backs off when the connection drops.
///
/// Jitter is on by default and matters more than it looks: a room full of
/// clients that lost the same wifi will otherwise all come back at the same
/// instant, and hit the server together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryConfig {
    /// How many times to retry before giving up.
    pub max_retries: usize,
    /// Delay before the first retry.
    pub initial_retry_delay: Duration,
    /// Ceiling the delay grows towards.
    pub max_retry_delay: Duration,
    /// What the delay is multiplied by after each failed attempt.
    pub backoff_factor: f32,
    /// Whether to spread retries out randomly.
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
    /// A retry policy from its parts.
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

    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    /// How long to wait before `attempt`, in milliseconds.
    ///
    /// Exponential in the attempt number, capped at
    /// [`Self::max_retry_delay`], then spread by up to 20% when jitter is on.
    #[must_use]
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
            delay = (delay as f32 * (1.0 + jitter_fraction())) as u64;
        }

        delay
    }

    /// Delay before the first retry.
    #[must_use]
    pub const fn initial_retry_delay(&self) -> Duration {
        self.initial_retry_delay
    }

    /// Ceiling the delay grows towards.
    #[must_use]
    pub const fn max_retry_delay(&self) -> Duration {
        self.max_retry_delay
    }
}

/// The configuration every client shares: where the server is, how to retry,
/// and the presenter token if there is one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseClientConfig {
    /// Base URL of the REST API.
    pub api_url: String,
    /// URL of the synchronisation socket.
    pub websocket_url: String,
    /// How to back off when the connection drops.
    pub retry: RetryConfig,
    /// Secret offered at registration so a client that is not on the server's
    /// own machine may still drive the deck. `None` on the usual local
    /// connection, where being local is credential enough.
    pub presenter_token: Option<Secret>,
}

impl BaseClientConfig {
    /// Points a client at `host:port`, over plain HTTP and `ws://`.
    #[must_use]
    pub fn new(host: &str, port: u16) -> Self {
        let api_url = format!("http://{host}:{port}");
        let websocket_url = format!("ws://{host}:{port}/api/ws");
        Self {
            api_url,
            websocket_url,
            retry: RetryConfig::default(),
            presenter_token: None,
        }
    }

    /// The usual case: a server on this machine, on port 8080.
    #[must_use]
    pub fn localhost() -> Self {
        Self::new("localhost", 8080)
    }

    /// Replaces the retry policy.
    #[must_use]
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Offers a presenter token on this connection.
    ///
    /// An empty token is dropped rather than sent: it can only be refused, and
    /// it comes from a flag or an environment variable that was set to nothing.
    /// [`Secret::new`] is what decides that, on both sides of the wire.
    #[must_use]
    pub fn with_presenter_token(mut self, token: Option<Secret>) -> Self {
        self.presenter_token = token.and_then(|token| Secret::new(token.expose()));
        self
    }
}

impl ClientConfig for BaseClientConfig {
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

    /// How often the server expects to hear from a client.
    pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
    /// How long to wait for a connection before giving up on it.
    pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
    /// How often a client sends a ping — shorter than the heartbeat, so a slow
    /// round trip does not read as a dead client.
    pub const PING_INTERVAL: Duration = Duration::from_secs(25);
}

#[cfg(test)]
#[allow(clippy::expect_used)]
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
    fn test_humantime_serialization() {
        let config = RetryConfig {
            max_retries: 5,
            initial_retry_delay: Duration::from_secs(2),
            max_retry_delay: Duration::from_secs(60),
            backoff_factor: 1.5,
            use_jitter: false,
        };

        let serialized = serde_json::to_string(&config).expect("serialize retry config");
        let deserialized =
            serde_json::from_str::<RetryConfig>(&serialized).expect("round-trip retry config");

        assert_eq!(config.max_retries, deserialized.max_retries);
        assert_eq!(config.initial_retry_delay, deserialized.initial_retry_delay);
        assert_eq!(config.max_retry_delay, deserialized.max_retry_delay);
        assert!((config.backoff_factor - deserialized.backoff_factor).abs() < f32::EPSILON);
        assert_eq!(config.use_jitter, deserialized.use_jitter);
    }

    #[test]
    fn test_humantime_parsing() {
        let json = r#"{
            "max_retries": 3,
            "initial_retry_delay": "1s",
            "max_retry_delay": "30s",
            "backoff_factor": 2.0,
            "use_jitter": true
        }"#;

        let config = serde_json::from_str::<RetryConfig>(json).expect("parse retry config");

        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_retry_delay, Duration::from_secs(1));
        assert_eq!(config.max_retry_delay, Duration::from_secs(30));
        assert!((config.backoff_factor - 2.0).abs() < f32::EPSILON);
        assert!(config.use_jitter);
    }
}
