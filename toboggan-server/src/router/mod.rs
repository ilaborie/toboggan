use std::path::PathBuf;

use axum::extract::State;
use axum::http::{Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::error;

use crate::{TobogganState, WebAssets};

mod api;
mod ws;

pub fn routes(assets_dir: Option<PathBuf>) -> Router<TobogganState> {
    routes_with_cors(None, assets_dir)
}

pub fn routes_with_cors(
    allowed_origins: Option<&[String]>,
    assets_dir: Option<PathBuf>,
) -> Router<TobogganState> {
    let cors = create_cors_layer(allowed_origins);

    let mut router = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/talk", get(api::get_talk))
                .route("/slides", get(api::get_slides))
                .route("/slides/{index}", get(api::get_slide_by_index))
                .route("/command", post(api::post_command))
                .route("/ws", get(ws::websocket_handler)),
        )
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    // Add local assets directory if provided (for presentation images/files)
    // Use /public to avoid conflict with embedded web assets
    if let Some(assets_dir) = assets_dir {
        router = router.nest_service("/public", ServeDir::new(assets_dir));
    }

    // Serve embedded web assets (catch-all for SPA)
    router = router.fallback(serve_embedded_web_assets);

    router
}

async fn serve_embedded_web_assets(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try to serve the requested file
    if let Some(content) = WebAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, mime.as_ref())],
            content.data,
        )
            .into_response();
    }

    // For SPA: serve index.html for all non-asset routes
    if let Some(index) = WebAssets::get("index.html") {
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html")],
            index.data,
        )
            .into_response();
    }

    // Fallback if index.html is not found
    (StatusCode::NOT_FOUND, "Not found").into_response()
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
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

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
                    error!("Invalid CORS origin format: {err}, falling back to Any");
                    cors = cors.allow_origin(Any);
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
