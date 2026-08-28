use std::path::PathBuf;

use axum::extract::State;
use axum::http::{HeaderValue, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router, middleware};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing::error;
use utoipa::openapi::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::TobogganState;

mod api;
mod origin;
mod pages;
mod presenter;
mod static_assets;
mod terminal_ws;
mod ws;

pub fn routes(assets_dir: Option<PathBuf>, openapi: OpenApi) -> Router<TobogganState> {
    routes_with_cors(None, assets_dir, openapi)
}

pub fn routes_with_cors(
    allowed_origins: Option<&[String]>,
    assets_dir: Option<PathBuf>,
    openapi: OpenApi,
) -> Router<TobogganState> {
    let cors = create_cors_layer(allowed_origins);
    // Applied per route rather than globally: a deck is meant to be embeddable,
    // and reading a slide from another page is the feature. Driving it is not.
    let guard = middleware::from_fn_with_state(
        origin::AllowedOrigins::new(allowed_origins),
        origin::guard_origin,
    );

    // Everything that moves the deck, opens a shell, names who is connected, or
    // spawns `typst`. `/ws` is here because the role it grants at the handshake
    // outlives the request, and it takes no `Presenter` extractor to speak for it.
    let privileged = Router::new()
        .route("/command", post(api::post_command))
        .route("/clients", get(api::get_clients))
        .route("/ws", get(ws::websocket_handler))
        .route("/terminal", get(terminal_ws::terminal_websocket_handler))
        .route_layer(guard.clone());

    let mut router = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/talk", get(api::get_talk))
                .route("/slides", get(api::get_slides))
                .route("/slides/{index}", get(api::get_slide_by_index))
                .merge(privileged),
        )
        .route("/health", get(health))
        // Server-rendered pages: landing page, the run/present app, the guide,
        // and the on-demand PDF download.
        .route("/", get(pages::homepage))
        .route("/run", get(pages::run_app))
        .route("/presenter", get(pages::presenter_app))
        .route("/guide", get(pages::guide))
        .route("/guide/public/{*path}", get(pages::guide_asset))
        // Rendering a PDF spawns `typst`, so another site does not get to ask.
        .route(
            "/download.pdf",
            get(pages::pdf::download_pdf).route_layer(guard),
        )
        // Slide overview: lazily generated on first hit, served from the cache.
        .route("/slides", get(pages::overview::slides_page))
        // Before the catch-all below, which would otherwise swallow it.
        .route(
            "/overview/slide/{index}",
            get(pages::overview::presented_thumbnail),
        )
        .route("/overview/{*path}", get(pages::overview::overview_asset))
        .merge(Scalar::with_url("/doc", openapi));

    // Add local assets directory if provided (for presentation images/files)
    // Use /public to avoid conflict with embedded web assets
    if let Some(assets_dir) = assets_dir {
        router = router.nest("/public", public_assets(assets_dir));
    }

    // Serve embedded web asset files only (hashed JS/CSS, favicon, manifest).
    // The `/` and `/run` routes own the HTML entry points.
    router = router.fallback(serve_embedded_web_assets);

    // Layers last, so they wrap every route above. `.layer()` only applies to
    // routes registered *before* it: with the tracing layer sitting up by `/api`,
    // none of the pages, `/public`, or the fallback were traced, and CORS missed
    // the last two as well.
    router.layer(cors).layer(TraceLayer::new_for_http())
}

/// Serves the deck's own `public/` directory, always revalidated.
///
/// These assets are author-editable and unhashed, so a stylesheet edited during
/// a talk must reach the browser on the next fetch. `ServeDir` sends `ETag` and
/// `Last-Modified` but no `Cache-Control`, which lets a cache pick its *own*
/// freshness lifetime and reuse a stale copy for hours without ever asking
/// (RFC 9111 §4.2.2) — and a hard reload does not rescue slide styles, which are
/// `@import`ed from a shadow root after load.
///
/// `no-cache` means "revalidate before reuse", not "do not store": paired with
/// the validators above, the steady-state cost is a conditional request per
/// asset answered `304` with no body. Do *not* use `no-store` here — it would
/// re-download every font on every slide change.
fn public_assets<S>(assets_dir: PathBuf) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .fallback_service(ServeDir::new(assets_dir))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
}

/// Serves a real embedded web asset file (hashed JS/CSS, favicon, manifest).
///
/// The `/` (homepage) and `/run` (present app) routes own the HTML entry points,
/// so this serves only real asset files and returns `404` for anything else —
/// never an `index.html` fallback, which would answer a mistyped asset URL with
/// a page instead of an error.
async fn serve_embedded_web_assets(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    match static_assets::WebAppAssets::get(path) {
        Some(content) => static_assets::bundle_asset_response(path, content.data.into_owned()),
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

async fn health(State(state): State<TobogganState>) -> impl IntoResponse {
    let start_time = std::time::Instant::now();
    let health_data = state.health().await;

    tracing::debug!(
        duration_ms = start_time.elapsed().as_millis(),
        active_clients = health_data.active_clients,
        "Health check completed"
    );

    Json(health_data)
}

fn create_cors_layer(allowed_origins: Option<&[String]>) -> CorsLayer {
    let mut cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    match allowed_origins {
        Some(origins) if !origins.is_empty() => {
            let parsed_origins: Result<Vec<_>, _> =
                origins.iter().map(|origin| origin.parse()).collect();

            match parsed_origins {
                Ok(origins) => {
                    tracing::info!(?origins, "CORS configured with specific origins");
                    cors = cors.allow_origin(origins);
                }
                Err(err) => {
                    // Falling back to `Any` turned a typo in a flag into an
                    // open server — the opposite of what the flag was set for.
                    error!("Invalid CORS origin format: {err}; allowing no origin");
                    cors = cors.allow_origin(Vec::<HeaderValue>::new());
                }
            }
        }
        _ => {
            cors = cors.allow_origin(Any);
            tracing::warn!("CORS configured to allow any origin - not recommended for production");
        }
    }

    cors
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::Request;
    use toboggan_core::{Command, Secret, Slide, Talk};
    use tower::ServiceExt as _;

    use super::*;
    use crate::PresenterAuth;
    use crate::services::{ClientService, TalkService};

    const REMOTE: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42)), 51_000);
    const LOCAL: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 51_000);

    fn test_router() -> Router {
        let talk = Talk::new("Test Talk").add_slide(Slide::cover("Cover"));
        let talk_service = TalkService::new(talk).expect("build talk service");
        let state = TobogganState::new(talk_service, ClientService::new(100), "sh".into());
        routes(None, OpenApi::default()).with_state(state)
    }

    /// Builds a request as if it had arrived over TCP from `peer`.
    ///
    /// Production inserts this extension via
    /// `into_make_service_with_connect_info`; a `oneshot` has no socket, so the
    /// test supplies what the socket would have.
    fn request_from(peer: SocketAddr, mut request: Request<Body>) -> Request<Body> {
        request.extensions_mut().insert(ConnectInfo(peer));
        request
    }

    /// The whole point of the presenter gate. `/api/terminal` spawns a shell on
    /// the machine running the server; a laptop on the same wifi must not be
    /// able to ask for one.
    #[tokio::test]
    async fn the_network_cannot_open_a_shell() {
        let response = test_router()
            .oneshot(request_from(
                REMOTE,
                Request::get("/api/terminal?cmd=id")
                    .body(Body::empty())
                    .expect("build request"),
            ))
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn the_network_cannot_move_the_deck() {
        let command = serde_json::to_vec(&Command::NextSlide).expect("serialize command");
        let response = test_router()
            .oneshot(request_from(
                REMOTE,
                Request::post("/api/command")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(command))
                    .expect("build request"),
            ))
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    /// …while the machine running the server is unaffected, which is the whole
    /// of the everyday workflow.
    #[tokio::test]
    async fn this_machine_still_drives_the_deck() {
        let command = serde_json::to_vec(&Command::NextSlide).expect("serialize command");
        let response = test_router()
            .oneshot(request_from(
                LOCAL,
                Request::post("/api/command")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(command))
                    .expect("build request"),
            ))
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// A server reachable from the network, with a token set. Everything below
    /// that names `GUARDED` is the `--presenter-token` feature end to end,
    /// which nothing exercised: `with_auth` had exactly one caller, and it was
    /// production. A regression in the extractor's token plumbing would have
    /// left every unit test green and remote presenting silently dead.
    fn guarded_router() -> Router {
        let talk = Talk::new("Test Talk").add_slide(Slide::cover("Cover"));
        let talk_service = TalkService::new(talk).expect("build talk service");
        let state = TobogganState::new(talk_service, ClientService::new(100), "sh".into())
            .with_auth(PresenterAuth::new(Secret::new(GUARDED)));
        routes(None, OpenApi::default()).with_state(state)
    }

    const GUARDED: &str = "s3cr3t";

    /// The point of the token: someone across the room, presenting.
    #[tokio::test]
    async fn the_network_drives_the_deck_with_the_token() {
        let command = serde_json::to_vec(&Command::NextSlide).expect("serialize command");
        let response = guarded_router()
            .oneshot(request_from(
                REMOTE,
                Request::post("/api/command")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {GUARDED}"))
                    .body(Body::from(command))
                    .expect("build request"),
            ))
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// A browser opening a `WebSocket` cannot set a header, so the token has
    /// nowhere to go but the query string. Both spellings must work.
    #[tokio::test]
    async fn the_token_travels_in_the_query_string_too() {
        let command = serde_json::to_vec(&Command::NextSlide).expect("serialize command");
        let response = guarded_router()
            .oneshot(request_from(
                REMOTE,
                Request::post(format!("/api/command?token={GUARDED}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(command))
                    .expect("build request"),
            ))
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// A token pasted with a trailing space, or percent-encoded as one, is the
    /// token. It used to be refused, with the same log line as no token at all.
    #[tokio::test]
    async fn a_token_pasted_with_whitespace_still_presents() {
        let command = serde_json::to_vec(&Command::NextSlide).expect("serialize command");
        let response = guarded_router()
            .oneshot(request_from(
                REMOTE,
                Request::post(format!("/api/command?token={GUARDED}%20"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(command))
                    .expect("build request"),
            ))
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn the_wrong_token_is_still_refused() {
        let command = serde_json::to_vec(&Command::NextSlide).expect("serialize command");
        let response = guarded_router()
            .oneshot(request_from(
                REMOTE,
                Request::post("/api/command?token=guess")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(command))
                    .expect("build request"),
            ))
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    /// Being on the presenter's machine is what `Presenter` asks about, and a
    /// page on any origin can make the browser ask from there. CORS does not
    /// stop it: the deck still moves whether or not the reply is readable.
    #[tokio::test]
    async fn another_site_cannot_move_the_deck_from_this_machine() {
        let command = serde_json::to_vec(&Command::NextSlide).expect("serialize command");
        let response = test_router()
            .oneshot(request_from(
                LOCAL,
                Request::post("/api/command")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::HOST, "127.0.0.1:8080")
                    .header(header::ORIGIN, "https://evil.example")
                    .body(Body::from(command))
                    .expect("build request"),
            ))
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    /// The same request from the page the server itself served. This is the
    /// everyday path, so the guard has to let it through untouched.
    #[tokio::test]
    async fn the_page_we_served_still_drives_the_deck() {
        let command = serde_json::to_vec(&Command::NextSlide).expect("serialize command");
        let response = test_router()
            .oneshot(request_from(
                LOCAL,
                Request::post("/api/command")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::HOST, "127.0.0.1:8080")
                    .header(header::ORIGIN, "http://127.0.0.1:8080")
                    .body(Body::from(command))
                    .expect("build request"),
            ))
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// A WebSocket handshake is not subject to CORS at all, which is how
    /// `/api/terminal` became a shell reachable from an ordinary web page. The
    /// upgrade is a plain GET, so the guard sees it before it becomes a socket.
    #[tokio::test]
    async fn another_site_cannot_open_a_shell_from_this_machine() {
        let response = test_router()
            .oneshot(request_from(
                LOCAL,
                Request::get("/api/terminal?cmd=id")
                    .header(header::HOST, "127.0.0.1:8080")
                    .header(header::ORIGIN, "https://evil.example")
                    .body(Body::empty())
                    .expect("build request"),
            ))
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    /// Watching is not gated: the audience URL has to keep working, or the
    /// gate would have turned a security fix into a broken feature.
    #[tokio::test]
    async fn the_network_can_still_watch() {
        for path in ["/api/talk", "/api/slides", "/health"] {
            let response = test_router()
                .oneshot(request_from(
                    REMOTE,
                    Request::get(path)
                        .body(Body::empty())
                        .expect("build request"),
                ))
                .await
                .expect("serve request");

            assert_eq!(response.status(), StatusCode::OK, "{path} should be public");
        }
    }

    /// The deck's own assets must never be reused without asking the server:
    /// an author editing `public/slide.css` mid-talk otherwise keeps seeing the
    /// copy the browser fetched an hour ago.
    #[tokio::test]
    async fn public_assets_are_revalidated() {
        let assets_dir = tempfile::tempdir().expect("temp assets dir");
        std::fs::write(assets_dir.path().join("slide.css"), "body { color: red }")
            .expect("write stylesheet");

        let response = public_assets::<()>(assets_dir.path().to_path_buf())
            .oneshot(
                Request::get("/slide.css")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("serve request");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CACHE_CONTROL)
                .expect("public assets must carry a cache policy"),
            "no-cache"
        );
        // `no-cache` is only cheap because the validators are still there.
        assert!(
            response.headers().contains_key(header::ETAG),
            "ServeDir should still send an ETag to revalidate against"
        );
    }
}
