use axum::extract::State;
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::warn;

use crate::TobogganState;
use toboggan_core::{Command, Notification};

mod ws;

pub fn routes() -> Router<TobogganState> {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

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
    Json(state.health())
}

async fn get_talk(State(state): State<TobogganState>) -> impl IntoResponse {
    let talk = state.talk();
    Json(json!({
        "talk": talk,
    }))
}

async fn get_slides(State(state): State<TobogganState>) -> impl IntoResponse {
    let slides = state.slides();
    Json(json!({
        "slides": slides,
    }))
}

async fn post_command(
    State(state): State<TobogganState>,
    Json(command): Json<Command>,
) -> impl IntoResponse {
    let result = state.handle_command(&command).await;
    if let Notification::Error { message, .. } = &result {
        warn!(?result, "{message}");
        (StatusCode::BAD_REQUEST, Json(result))
    } else {
        (StatusCode::OK, Json(result))
    }
}
