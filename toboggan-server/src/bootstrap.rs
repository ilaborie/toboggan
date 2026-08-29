use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use toboggan_cli::mermaid::MermaidRenderer;
use toboggan_core::Talk;
use tracing::{info, instrument, warn};
use utoipa::openapi::OpenApi;

use crate::{
    ClientService, PresenterAuth, ServerSettings, Settings, TalkService, TobogganState,
    WatchConfig, WatchTarget, routes_for_shots, routes_with_cors, start_watch_task,
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
            target: WatchTarget::TalkFile(settings.talk.clone()),
            reload: Box::new(move || load_talk_sync(&reload_path)),
        }
    });

    let mermaid = MermaidRenderer::from_config(settings.mermaid_config.as_deref())
        .context("Loading the Mermaid config")?;

    launch_with_talk(talk, settings.server, watch, mermaid).await
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
    settings: ServerSettings,
    watch: Option<WatchConfig>,
    mermaid: MermaidRenderer,
) -> anyhow::Result<()> {
    info!(?settings, "launching server...");
    let ServerSettings {
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

    let talk_service = TalkService::new(talk).context("build talk service")?;
    let client_service = ClientService::new(max_clients);
    let cleanup_service = client_service.clone();
    let terminal_shell = settings.resolve_shell();
    info!(%terminal_shell, "Embedded terminals will use this shell");
    let auth = PresenterAuth::new(settings.presenter_token.clone());
    report_access_posture(host, port, &auth);
    let state = TobogganState::new(talk_service, client_service, terminal_shell.into())
        .with_auth(auth)
        .with_mermaid(mermaid)
        .with_overview(crate::OverviewOptions {
            renderer: settings.thumbnail_renderer,
            browser: settings.browser.clone(),
            // The same `public/` the router serves, so a slide's pictures
            // resolve in the private server the shots are taken against as well
            // as in the one the room is looking at.
            public_dir: settings.public_dir.clone(),
            ..crate::OverviewOptions::default()
        });

    // A pre-generated overview directory (`--thumbnails-dir`) seeds the cache as
    // ready and there is nothing to draw. Otherwise the deck is photographed
    // now, unless `--no-eager-thumbnails` asks for the older behaviour of
    // waiting for the first request that wants a picture.
    if let Some(thumbnails_dir) = settings.thumbnails_dir.clone() {
        state.seed_thumbnails_dir(thumbnails_dir).await;
    } else if !settings.no_eager_thumbnails {
        // Warm them now rather than on the first request. The presenter view
        // wants a picture of the next slide the moment it opens, and
        // photographing a deck takes seconds — long enough that a speaker who
        // opens the view as the room settles would watch it fill in.
        //
        // `ensure` spawns the generation and returns, so this costs the startup
        // path a lock rather than a browser launch; everything that asks in the
        // meantime is told `Pending` and polls, exactly as it does today.
        info!("Photographing the deck for the slide overview");
        state.ensure_thumbnails().await;
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

    // Opened here rather than straight after `bind`: the socket accepts into the
    // backlog from that moment, so a browser launched earlier could issue its
    // `GET /run` and then sit on it, unanswered, until the router below existed
    // — with the deck's own parse and the OpenAPI build in between.
    if settings.open || settings.open_presenter {
        // The presenter view goes second so it lands on top: the deck belongs on
        // the projector, and the window the speaker is left looking at should be
        // the one with their notes in it.
        let mut pages = vec![browse_url(host, port, "")];
        if settings.open_presenter {
            pages.push(browse_url(host, port, "presenter"));
        }
        for url in pages {
            info!(%url, "Opening in the default browser");
            tokio::task::spawn_blocking(move || {
                if let Err(err) = open::that(&url) {
                    warn!(%url, %err, "Could not open the browser");
                }
            });
        }
    }

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

/// A throwaway server on a loopback port the OS picked.
///
/// What [`crate::shoot_slides`] photographs. A headless browser needs the deck
/// over HTTP — the whole point of shooting `/run` is that the thumbnail
/// and the slide are the same rendering — and it is the *live* server that wants
/// thumbnails, so the obvious move is to point the browser at it.
///
/// It cannot be. The live server serves the deck *as presented*, with
/// `hidden_in = ["web"]` slides dropped and the rest renumbered, while
/// `thumb-NNNN.png` is named over the deck *as authored* — so photographing by
/// index through it reads the wrong slide for every index past a hidden one, and
/// no amount of care at the call site fixes that. A private server can be handed
/// the unfiltered talk instead; see `shots::shootable`. Two lesser reasons come
/// free: a server bound to `0.0.0.0` would be photographed over its public
/// address, and forty-four navigations would go through the room's own server
/// while a talk is running.
///
/// A second, private server sidesteps all three, and gives the *offline* path —
/// `toboggan thumbnails`, which has no server at all — exactly the same code.
///
/// It has no watcher and no `--open`, allows exactly one CORS origin (its own),
/// and serves [`routes_for_shots`] rather than the full router — on loopback
/// every client is a presenter, so the shell behind `/api/terminal` would
/// otherwise be a port-scan away for as long as the shots take.
///
/// # Errors
/// Returns an error if the loopback port cannot be bound, the talk service
/// cannot be built.
pub async fn serve_ephemeral(
    talk: Talk,
    mermaid: MermaidRenderer,
    public_dir: Option<std::path::PathBuf>,
) -> anyhow::Result<EphemeralServer> {
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .context("Binding an ephemeral loopback port")?;
    let addr = listener.local_addr().context("Reading the bound address")?;

    let talk_service = TalkService::new(talk).context("build talk service")?;
    let state =
        TobogganState::new(talk_service, ClientService::new(1), "sh".into()).with_mermaid(mermaid);
    // Its own origin and no other. Nothing cross-origin is ever asked of this
    // server — the page doing the asking is the page it served — and the `None`
    // that would otherwise be natural here means *any* origin, which is both a
    // warning in the log and a small open door on the author's own machine.
    let own_origin = [format!("http://{addr}")];
    let router = routes_for_shots(Some(&own_origin), public_dir).with_state(state);

    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let serving = tokio::spawn(async move {
        let served = axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        // A dropped sender resolves this too, so forgetting to `stop()` leaks
        // the task only until the handle goes out of scope.
        .with_graceful_shutdown(async move {
            let _ = stopped.await;
        })
        .await;
        if let Err(err) = served {
            warn!(%err, "Ephemeral server stopped early");
        }
    });

    info!(%addr, "Serving the deck privately for rendering");
    Ok(EphemeralServer {
        addr,
        stop,
        serving,
    })
}

/// A running [`serve_ephemeral`]. Shuts down when dropped; `stop` waits for it.
pub struct EphemeralServer {
    addr: SocketAddr,
    stop: tokio::sync::oneshot::Sender<()>,
    serving: tokio::task::JoinHandle<()>,
}

impl EphemeralServer {
    /// Where to point a browser.
    #[must_use]
    pub const fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The origin to prefix a path with, e.g. `http://127.0.0.1:52341`.
    #[must_use]
    pub fn origin(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Asks the server to stop and waits for it to.
    pub async fn stop(self) {
        let _ = self.stop.send(());
        let _ = self.serving.await;
    }
}

/// Says out loud who will be able to drive the deck.
///
/// Worth a line in the log because the answer changes with `--host`, and the
/// case that changes it is the one an author reaches for five minutes before a
/// talk — "let the room open it on their laptops" — without meaning to also
/// offer the room a shell.
fn report_access_posture(host: IpAddr, port: u16, auth: &PresenterAuth) {
    if host.is_loopback() {
        info!("Only this machine can reach the server, so every client presents");
    } else if let Some(token) = auth.token_for_link() {
        // Printed, because two clients' doc comments said the server prints the
        // presenter link and nothing did. Assembling `?token=…` by hand is how
        // a token picks up the stray whitespace and encoding that used to make
        // it fail to match.
        info!(
            "Reachable from the network: clients present from this machine, \
             or with the presenter token"
        );
        // Not `browse_url`: that rewrites a wildcard bind to loopback, which is
        // right for `--open` on this machine and useless in a link whose whole
        // purpose is to be opened from another one. A wildcard bind has no one
        // address to name, so the host is left for the operator to fill in.
        let authority = match host {
            IpAddr::V4(addr) if addr.is_unspecified() => "<this-machine>".to_owned(),
            IpAddr::V6(addr) if addr.is_unspecified() => "<this-machine>".to_owned(),
            addr => SocketAddr::new(addr, port).to_string(),
        };
        let port = if authority == "<this-machine>" {
            format!(":{port}")
        } else {
            String::new()
        };
        info!("Presenter link: http://{authority}{port}/run?token={token}");
    } else {
        warn!(
            "Reachable from the network: remote clients are read-only and cannot open \
             terminals. Pass --presenter-token to let one drive the deck."
        );
    }
}

/// Builds a URL `--open` hands to the browser.
///
/// Two things the bind address cannot supply directly: a wildcard bind
/// (`0.0.0.0` / `::`) is not an address a client can connect to, so it becomes
/// loopback; and an IPv6 literal needs brackets in a URL authority, which
/// `SocketAddr`'s `Display` adds and plain interpolation does not.
fn browse_url(host: IpAddr, port: u16, path: &str) -> String {
    let host = match host {
        IpAddr::V4(addr) if addr.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(addr) if addr.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        addr => addr,
    };
    format!("http://{}/{path}", SocketAddr::new(host, port))
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

    // Actually enforce the timeout the flag advertises. Axum's graceful shutdown
    // waits for every open connection to close, and a presentation WebSocket
    // never closes on its own — so Ctrl+C hung for as long as one browser tab
    // stayed open, however `--shutdown-timeout` was set.
    tokio::spawn(async move {
        tokio::time::sleep(timeout).await;
        warn!(
            timeout_secs = timeout.as_secs(),
            "graceful shutdown timed out with connections still open; exiting"
        );
        std::process::exit(0);
    });

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
            browse_url(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080, ""),
            "http://127.0.0.1:8080/"
        );
        assert_eq!(
            browse_url(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 8080, ""),
            "http://[::1]:8080/"
        );
        // An IPv6 literal needs brackets in the URL authority.
        assert_eq!(
            browse_url(IpAddr::V6(Ipv6Addr::LOCALHOST), 3000, ""),
            "http://[::1]:3000/"
        );
        assert_eq!(
            browse_url(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)), 80, ""),
            "http://192.168.1.10:80/"
        );
        assert_eq!(
            browse_url(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080, "presenter"),
            "http://127.0.0.1:8080/presenter"
        );
    }

    /// The shot server is the one place the presenter gate cannot help: it lives
    /// on loopback, where every client is granted the presenter role. It used to
    /// be handed the full router, so for as long as a deck took to photograph,
    /// any other process on the machine that found the port could ask it for a
    /// shell. Driven against a real socket rather than the router value, so that
    /// pointing `serve_ephemeral` back at the full router fails here too.
    #[tokio::test]
    #[allow(clippy::expect_used)]
    async fn a_shot_server_offers_the_deck_and_nothing_else() {
        use toboggan_core::Slide;

        let talk = Talk::new("Shot Deck").add_slide(Slide::cover("Cover"));
        let server = serve_ephemeral(talk, MermaidRenderer::default(), None)
            .await
            .expect("start the ephemeral server");

        for path in [
            "/api/terminal?cmd=id",
            "/api/clients",
            "/download.pdf",
            "/presenter",
        ] {
            assert_eq!(
                status_of(server.addr(), path).await,
                404,
                "{path} is reachable on the shot server"
            );
        }
        // …while what the shot page asks for still answers.
        for path in ["/api/talk", "/api/slides/0", "/run"] {
            assert_eq!(
                status_of(server.addr(), path).await,
                200,
                "{path} does not answer on the shot server"
            );
        }

        server.stop().await;
    }

    /// One request over a real socket, without pulling in an HTTP client.
    #[allow(clippy::expect_used)]
    async fn status_of(addr: SocketAddr, path: &str) -> u16 {
        use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

        let mut stream = tokio::net::TcpStream::connect(addr)
            .await
            .expect("connect to the ephemeral server");
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        stream
            .write_all(request.as_bytes())
            .await
            .expect("send the request");

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .expect("read the response");
        String::from_utf8_lossy(&response)
            .split_whitespace()
            .nth(1)
            .expect("a status line")
            .parse()
            .expect("a status code")
    }
}
