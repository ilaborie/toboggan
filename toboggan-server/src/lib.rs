use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use toboggan_core::Talk;
use tracing::{info, instrument, warn};

mod settings;
pub use self::settings::*;

mod error;
pub use self::error::*;

mod domain;
pub use self::domain::*;

mod state;
pub use self::state::*;

mod router;
pub use self::router::{routes_with_cors, *};

mod static_assets;
pub use self::static_assets::*;

#[doc(hidden)]
#[instrument]
pub async fn launch(settings: Settings) -> anyhow::Result<()> {
    info!(?settings, "launching server...");
    let Settings {
        host,
        port,
        ref talk,
        max_clients,
        ..
    } = settings;

    let talk = load_talk(talk).context("Loading talk")?;

    let addr = SocketAddr::from((host, port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Connecting to {addr} ..."))?;

    let state = TobogganState::new(talk, max_clients).context("build state")?;

    let cleanup_state = state.clone();
    let cleanup_interval = settings.cleanup_interval();
    tokio::spawn(async move {
        cleanup_state.cleanup_clients_task(cleanup_interval).await;
        info!("Cleanup task completed");
    });

    let router = routes_with_cors(
        settings.allowed_origins.as_deref(),
        settings.assets_dir.clone(),
    )
    .with_state(state);
    let shutdown_signal = setup_shutdown_signal(settings.shutdown_timeout());

    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_signal)
        .await
        .context("Axum server")?;

    info!("Server shutdown complete");

    Ok(())
}

#[instrument]
fn load_talk(path: &Path) -> anyhow::Result<Talk> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Reading talk file {}", path.display()))?;
    let result = toml::from_str(&content).context("Parsing talk")?;
    Ok(result)
}

async fn setup_shutdown_signal(timeout: Duration) {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            warn!("Failed to install Ctrl+C handler: {err}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => {
                warn!("Failed to install SIGTERM handler: {err}");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown...");
        }
        () = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown...");
        }
    }

    info!(
        "Waiting up to {} seconds for graceful shutdown",
        timeout.as_secs()
    );

    info!("Shutdown signal processed, server will now terminate gracefully");
}
