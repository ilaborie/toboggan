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

    /// Optional local public folder for presentation files (served at /public/)
    /// Example: --public-dir ./public for images, videos, etc.
    #[clap(long, env = "TOBOGGAN_PUBLIC_DIR")]
    pub public_dir: Option<PathBuf>,
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

    /// # Errors
    /// Returns error if configuration is invalid
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

        if let Some(ref assets_dir) = self.public_dir {
            if !assets_dir.exists() {
                return Err(format!(
                    "Assets directory does not exist: {}",
                    assets_dir.display()
                ));
            }
            if !assets_dir.is_dir() {
                return Err(format!(
                    "Assets path is not a directory: {}",
                    assets_dir.display()
                ));
            }
        }

        Ok(())
    }
}
