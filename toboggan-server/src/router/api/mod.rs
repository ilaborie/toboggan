use std::time::Instant;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use toboggan_core::{Command, Notification, SlideId, SlidesResponse, TalkResponse};
use tracing::{info, warn};

use crate::TobogganState;

pub(super) async fn get_talk(State(state): State<TobogganState>) -> impl IntoResponse {
    let talk = state.talk().clone();
    let result = TalkResponse::from(talk);

    Json(result)
}

pub(super) async fn get_slides(State(state): State<TobogganState>) -> impl IntoResponse {
    let slides = state.slides();
    let result = SlidesResponse { slides };

    Json(result)
}

pub(super) async fn get_slide_by_id(
    State(state): State<TobogganState>,
    Path(id): Path<SlideId>,
) -> impl IntoResponse {
    state.slide_by_id(id).map(Json).ok_or(StatusCode::NOT_FOUND)
}

pub(super) async fn post_command(
    State(state): State<TobogganState>,
    Json(command): Json<Command>,
) -> impl IntoResponse {
    let start_time = Instant::now();

    let result = state.handle_command(&command).await;
    let duration_ms = start_time.elapsed().as_millis();
    match &result {
        Notification::Error { message, .. } => {
            warn!(?command, %message, ?duration_ms, "Command failed");
        }
        notification => {
            info!(
                ?command,
                ?duration_ms,
                ?notification,
                "Command processed successfully"
            );
        }
    }

    Json(result)
}
