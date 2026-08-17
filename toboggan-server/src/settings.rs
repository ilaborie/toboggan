use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::Duration;

/// Everything the server needs once it already has a [`crate::Talk`].
///
/// Split out from [`Settings`] because `launch_with_talk` — the entry point the
/// unified CLI's build+serve uses — has the talk in memory and never reads
/// `talk` or `watch`. Keeping them in one struct meant those two fields were
/// filled with placeholders on that path, and the startup log dutifully reported
/// the placeholders as if they were configuration.
#[derive(Debug, clap::Parser)]
pub struct ServerSettings {
    /// The host to bind to
    #[clap(long, env = "TOBOGGAN_HOST", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub host: IpAddr,

    /// The port to bind to
    #[clap(long, env = "TOBOGGAN_PORT", default_value_t = 8080)]
    pub port: u16,

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

    /// Optional directory of generated thumbnails + overview.html.
    /// Served at /overview/ and linked from the homepage as /slides.
    #[clap(long, env = "TOBOGGAN_THUMBNAILS_DIR")]
    pub thumbnails_dir: Option<PathBuf>,

    /// Shell to spawn for embedded terminals (e.g. `/opt/homebrew/bin/fish`).
    /// Defaults to the `SHELL` environment variable, then `sh`.
    #[clap(long, env = "TOBOGGAN_SHELL")]
    pub shell: Option<String>,

    /// Open the presentation in the default browser once the server is ready
    #[clap(long, env = "TOBOGGAN_OPEN")]
    pub open: bool,

    /// Also open the presenter view — notes, next slide, and a timer
    ///
    /// Two windows for one talk: this one on your screen, the deck on the
    /// projector. Implies `--open`.
    #[clap(long, env = "TOBOGGAN_OPEN_PRESENTER")]
    pub open_presenter: bool,

    /// Secret that lets a client **not** on this machine drive the deck.
    ///
    /// Only needed when the server is reachable from the network: a connection
    /// from this machine always presents. Remote clients pass it as
    /// `?token=…` (`/run?token=…`, and the WebSocket picks it up from there).
    #[clap(long, env = "TOBOGGAN_PRESENTER_TOKEN")]
    pub presenter_token: Option<String>,
}

/// `toboggan serve`'s settings: which talk file to serve, and how.
#[derive(Debug, clap::Parser)]
pub struct Settings {
    #[clap(flatten)]
    pub server: ServerSettings,

    /// The talk file to serve
    pub talk: PathBuf,

    /// Enable watch mode to automatically reload the talk file when it changes
    #[clap(long, env = "TOBOGGAN_WATCH")]
    pub watch: bool,
}

impl ServerSettings {
    /// Resolves the shell for embedded terminals: explicit `--shell`, else `$SHELL`, else `sh`.
    #[must_use]
    pub fn resolve_shell(&self) -> String {
        self.shell
            .clone()
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "sh".to_owned())
    }

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
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.max_clients > 0, "max_clients must be greater than 0");
        anyhow::ensure!(
            self.heartbeat_interval_secs > 0,
            "heartbeat_interval_secs must be greater than 0"
        );

        if let Some(assets_dir) = &self.public_dir {
            anyhow::ensure!(
                assets_dir.exists(),
                "Assets directory does not exist: {}",
                assets_dir.display()
            );
            anyhow::ensure!(
                assets_dir.is_dir(),
                "Assets path is not a directory: {}",
                assets_dir.display()
            );
        }

        Ok(())
    }
}

impl Settings {
    /// # Errors
    /// Returns an error if the configuration or the talk file is invalid.
    pub fn validate(&self) -> anyhow::Result<()> {
        self.server.validate()?;

        anyhow::ensure!(
            self.talk.exists(),
            "Talk file does not exist: {}",
            self.talk.display()
        );
        anyhow::ensure!(
            self.talk.extension().is_some_and(|ext| ext == "toml"),
            "Talk file must have .toml extension"
        );

        Ok(())
    }
}
