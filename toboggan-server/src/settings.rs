use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, clap::Parser)]
pub struct Settings {
    /// The host to bind to
    #[clap(long, env = "TOBOGGAN_HOST", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub host: IpAddr,

    /// The port to bind to
    #[clap(long, env = "TOBOGGAN_PORT", default_value_t = 8080)]
    pub port: u16,

    /// The talk file to serve
    pub talk: PathBuf,

    /// Maximum number of concurrent WebSocket clients
    #[clap(long, env = "TOBOGGAN_MAX_CLIENTS", default_value_t = 100)]
    pub max_clients: usize,

    /// WebSocket heartbeat interval in seconds
    #[clap(long, env = "TOBOGGAN_HEARTBEAT_INTERVAL", default_value_t = 30)]
    pub heartbeat_interval_secs: u64,

    /// Graceful shutdown timeout in seconds
    #[clap(long, env = "TOBOGGAN_SHUTDOWN_TIMEOUT", default_value_t = 30)]
    pub shutdown_timeout_secs: u64,

    /// Client cleanup interval in seconds
    #[clap(long, env = "TOBOGGAN_CLEANUP_INTERVAL", default_value_t = 60)]
    pub cleanup_interval_secs: u64,

    /// Allowed CORS origins (comma-separated)
    #[clap(long, env = "TOBOGGAN_CORS_ORIGINS", value_delimiter = ',')]
    pub allowed_origins: Option<Vec<String>>,

    /// Enable request tracing
    #[clap(long, env = "TOBOGGAN_ENABLE_TRACING", action = clap::ArgAction::SetTrue)]
    pub enable_tracing: bool,

    /// Log level
    #[clap(long, env = "TOBOGGAN_LOG_LEVEL", default_value = "info")]
    pub log_level: String,
}

impl Settings {
    #[must_use]
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_secs(self.heartbeat_interval_secs)
    }

    #[must_use]
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.shutdown_timeout_secs)
    }

    #[must_use]
    pub fn cleanup_interval(&self) -> Duration {
        Duration::from_secs(self.cleanup_interval_secs)
    }

    /// Validates the settings configuration
    ///
    /// # Errors
    /// Returns an error if any setting is invalid:
    /// - `max_clients` is 0
    /// - `heartbeat_interval_secs` is 0
    /// - talk file doesn't exist or isn't a .toml file
    pub fn validate(&self) -> Result<(), String> {
        if self.max_clients == 0 {
            return Err("max_clients must be greater than 0".to_string());
        }

        if self.heartbeat_interval_secs == 0 {
            return Err("heartbeat_interval_secs must be greater than 0".to_string());
        }

        if !self.talk.exists() {
            return Err(format!("Talk file does not exist: {}", self.talk.display()));
        }

        if self.talk.extension().is_none_or(|ext| ext != "toml") {
            return Err("Talk file must have .toml extension".to_string());
        }

        Ok(())
    }
}
