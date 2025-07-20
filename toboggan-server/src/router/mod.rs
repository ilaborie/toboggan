use axum::extract::State;
use axum::http::Method;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, warn};

use crate::TobogganState;
use toboggan_core::{Command, Notification};

mod responses;
mod ws;

pub use responses::*;

pub fn routes() -> Router<TobogganState> {
    routes_with_cors(None)
}

pub fn routes_with_cors(allowed_origins: Option<&[String]>) -> Router<TobogganState> {
    let cors = create_cors_layer(allowed_origins);

    Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/talk", get(get_talk))
                .route("/slides", get(get_slides))
                .route("/command", post(post_command))
                .route("/ws", get(ws::websocket_handler)),
        )
        .layer(TraceLayer::new_for_http())
        .route("/health", get(health))
        .layer(cors)
}

async fn health(State(state): State<TobogganState>) -> impl IntoResponse {
    let start_time = std::time::Instant::now();
    let health_data = state.health();

    tracing::debug!(
        duration_ms = start_time.elapsed().as_millis(),
        active_clients = health_data.active_clients,
        "Health check completed"
    );

    ApiResponse::new(health_data)
}

async fn get_talk(State(state): State<TobogganState>) -> impl IntoResponse {
    let talk = state.talk().clone();
    ApiResponse::new(TalkResponse { talk })
}

async fn get_slides(State(state): State<TobogganState>) -> impl IntoResponse {
    // Use Arc clone instead of deep clone for better performance
    let slides = state.slides_arc().clone();
    ApiResponse::new(SlidesResponse {
        slides: (*slides).clone(),
    })
}

async fn post_command(
    State(state): State<TobogganState>,
    Json(command): Json<Command>,
) -> impl IntoResponse {
    let start_time = std::time::Instant::now();

    match state.handle_command(&command).await {
        Notification::Error { ref message, .. } => {
            warn!(
                ?command,
                duration_ms = start_time.elapsed().as_millis(),
                "Command failed: {message}"
            );
            ErrorResponse::bad_request(message.clone()).into_response()
        }
        notification => {
            tracing::info!(
                ?command,
                duration_ms = start_time.elapsed().as_millis(),
                "Command processed successfully"
            );
            ApiResponse::new(notification).into_response()
        }
    }
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
