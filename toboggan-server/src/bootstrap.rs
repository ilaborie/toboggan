use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use toboggan_core::Talk;
use tracing::{info, instrument, warn};
use utoipa::openapi::OpenApi;

use crate::{
    ClientService, Settings, TalkService, TobogganState, WatchConfig, routes_with_cors,
    start_watch_task,
};

/// Loads the talk from `settings.talk` and serves it.
///
/// When `settings.watch` is set, the single `.toml` file is watched and the talk
/// is hot-swapped on change. To serve an in-memory talk (e.g. built from a
/// folder) use [`launch_with_talk`].
#[doc(hidden)]
#[instrument]
pub async fn launch(settings: Settings) -> anyhow::Result<()> {
    let talk = load_talk(&settings.talk).await.context("Loading talk")?;

    let watch = settings.watch.then(|| {
        let reload_path = settings.talk.clone();
        WatchConfig {
            paths: vec![settings.talk.clone()],
            recursive: false,
            reload: Box::new(move || load_talk_sync(&reload_path)),
        }
    });

    launch_with_talk(talk, settings, watch).await
}

/// Serves an already-built [`Talk`], optionally watching a path for reloads.
///
/// This is the shared serving core: `launch` uses it after reading a `.toml`
/// file, and the unified CLI's build+serve uses it with a talk parsed from a
/// folder plus a recursive [`WatchConfig`].
///
/// # Errors
/// Returns an error if the address cannot be bound, the talk has no slides, the
/// watcher cannot start, or the HTTP server fails.
#[doc(hidden)]
pub async fn launch_with_talk(
    talk: Talk,
    settings: Settings,
    watch: Option<WatchConfig>,
) -> anyhow::Result<()> {
    info!(?settings, "launching server...");
    let Settings {
        host,
        port,
        max_clients,
        ..
    } = settings;

    let addr = SocketAddr::from((host, port));
    info!(?addr, "Using address");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Connecting to {addr} ..."))?;

    if settings.open {
        let url = browse_url(host, port);
        info!(%url, "Opening presentation in the default browser");
        tokio::task::spawn_blocking(move || {
            if let Err(err) = open::that(&url) {
                warn!(%url, %err, "Could not open the browser");
            }
        });
    }

    let talk_service = TalkService::new(talk).context("build talk service")?;
    let client_service = ClientService::new(max_clients);
    let cleanup_service = client_service.clone();
    let terminal_shell = settings.resolve_shell();
    info!(%terminal_shell, "Embedded terminals will use this shell");
    let state = TobogganState::new(talk_service, client_service, terminal_shell.into());

    // A pre-generated overview directory (`--thumbnails-dir`) seeds the cache as
    // ready; otherwise the overview is generated lazily on the first request.
    if let Some(thumbnails_dir) = settings.thumbnails_dir.clone() {
        state.seed_thumbnails_dir(thumbnails_dir).await;
    }

    let cleanup_interval = settings.cleanup_interval();
    tokio::spawn(async move {
        cleanup_service.cleanup_clients_task(cleanup_interval).await;
        info!("Cleanup task completed");
    });

    if let Some(watch) = watch {
        start_watch_task(watch, state.clone()).context("Starting watcher")?;
    }

    let openapi = create_openapi()?;

    let router = routes_with_cors(
        settings.allowed_origins.as_deref(),
        settings.public_dir.clone(),
        openapi,
    )
    .with_state(state);
    let shutdown_signal = setup_shutdown_signal(settings.shutdown_timeout());

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal)
    .await
    .context("Axum server")?;

    info!("Server shutdown complete");

    Ok(())
}

/// Builds the URL `--open` hands to the browser.
///
/// Two things the bind address cannot supply directly: a wildcard bind
/// (`0.0.0.0` / `::`) is not an address a client can connect to, so it becomes
/// loopback; and an IPv6 literal needs brackets in a URL authority, which
/// `SocketAddr`'s `Display` adds and plain interpolation does not.
fn browse_url(host: IpAddr, port: u16) -> String {
    let host = match host {
        IpAddr::V4(addr) if addr.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(addr) if addr.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        addr => addr,
    };
    format!("http://{}/", SocketAddr::new(host, port))
}

#[instrument]
async fn load_talk(path: &Path) -> anyhow::Result<Talk> {
    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Reading talk file {}", path.display()))?;
    let result = toml::from_str(&content).context("Parsing talk")?;
    Ok(result)
}

/// Synchronous talk load used by the single-file reload watcher.
fn load_talk_sync(path: &Path) -> anyhow::Result<Talk> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Reading talk file {}", path.display()))?;
    toml::from_str(&content).context("Parsing talk")
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

/// The bundled `OpenAPI` document as a JSON string.
///
/// Exposed so the unified CLI's `openapi` subcommand can emit it without
/// starting the server.
#[must_use]
pub fn openapi_json() -> &'static str {
    include_str!("../openapi.json")
}

fn create_openapi() -> anyhow::Result<OpenApi> {
    let openapi = serde_json::from_str(openapi_json()).context("reading openapi.json file")?;
    Ok(openapi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::expect_used)]
    fn test_create_openapi() {
        let result = create_openapi();

        assert!(result.is_ok(), "create_openapi should succeed: {result:?}");

        let openapi = result.expect("should have OpenApi");

        // Check that paths are present (from the generated openapi.yml)
        assert!(
            !openapi.paths.paths.is_empty(),
            "should have API paths from openapi.yml"
        );

        // Check that schemas are present
        assert!(
            openapi.components.is_some(),
            "should have component schemas"
        );
    }

    #[test]
    fn browse_url_is_reachable_and_bracketed() {
        // A wildcard bind is not connectable; loopback of the same family is.
        assert_eq!(
            browse_url(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080),
            "http://127.0.0.1:8080/"
        );
        assert_eq!(
            browse_url(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 8080),
            "http://[::1]:8080/"
        );
        // An IPv6 literal needs brackets in the URL authority.
        assert_eq!(
            browse_url(IpAddr::V6(Ipv6Addr::LOCALHOST), 3000),
            "http://[::1]:3000/"
        );
        assert_eq!(
            browse_url(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)), 80),
            "http://192.168.1.10:80/"
        );
    }
}
