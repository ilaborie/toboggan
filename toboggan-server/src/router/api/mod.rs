use std::time::Instant;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use toboggan_core::{Command, Notification, SlidesResponse, TalkResponse};
use tracing::{info, warn};

use crate::TobogganState;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct TalkParam {
    #[serde(default)]
    footer: bool,
}

pub(super) async fn get_talk(
    State(state): State<TobogganState>,
    Query(param): Query<TalkParam>,
) -> impl IntoResponse {
    let talk = state.talk().clone();
    let mut result = TalkResponse::from(talk);
    if !param.footer {
        result.footer.take();
    }

    Json(result)
}

pub(super) async fn get_slides(State(state): State<TobogganState>) -> impl IntoResponse {
    let slides = state.slides().to_vec();
    let result = SlidesResponse { slides };

    Json(result)
}

pub(super) async fn get_slide_by_index(
    State(state): State<TobogganState>,
    Path(index): Path<usize>,
) -> impl IntoResponse {
    state
        .slide_by_index(index)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
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
