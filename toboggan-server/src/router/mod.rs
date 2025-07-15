use axum::extract::State;
use axum::http::Method;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use jiff::Timestamp;
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::TobogganState;

pub fn routes() -> Router<TobogganState> {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/talk", get(get_talk))
                .route("/slides", get(get_slides)),
        )
        .layer(TraceLayer::new_for_http())
        .route("/health", get(health))
        .layer(cors)
}

async fn health(State(state): State<TobogganState>) -> impl IntoResponse {
    let started_at = state.started_at;
    let elasped = Timestamp::now() - started_at;
    let name = state.talk.title.to_string();

    let health = json!({
        "status": "OK",
        "started_at": started_at,
        "elapsed": elasped,
        "talk": name
    });
    Json(health)
}

async fn get_talk(State(state): State<TobogganState>) -> impl IntoResponse {
    let talk = state.talk.as_ref().clone();
    Json(json!({
        "talk": talk,
    }))
}

async fn get_slides(State(state): State<TobogganState>) -> impl IntoResponse {
    let slides = state.slides.as_ref().clone();
    Json(json!({
        "slides": slides,
    }))
}
