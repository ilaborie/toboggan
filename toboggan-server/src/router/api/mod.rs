use std::time::Instant;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use toboggan_core::{
    ClientsResponse, Command, Notification, SlideId, SlidesResponse, TalkResponse,
};
use tracing::{info, warn};

use crate::TobogganState;
use crate::services::{ClientService, TalkService};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct TalkParam {
    #[serde(default)]
    footer: bool,
    #[serde(default)]
    head: bool,
}

pub(super) async fn get_talk(
    State(talk_service): State<TalkService>,
    Query(param): Query<TalkParam>,
) -> impl IntoResponse {
    let talk = talk_service.talk().await;
    let mut result = TalkResponse::from(talk);
    if !param.footer {
        result.footer.take();
    }
    if !param.head {
        result.head.take();
    }

    Json(result)
}

pub(super) async fn get_slides(State(talk_service): State<TalkService>) -> impl IntoResponse {
    let slides = talk_service.slides().await;
    let result = SlidesResponse { slides };

    Json(result)
}

pub(super) async fn get_slide_by_index(
    State(talk_service): State<TalkService>,
    Path(slide_id): Path<SlideId>,
) -> impl IntoResponse {
    talk_service
        .slide_by_index(slide_id)
        .await
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

pub(super) async fn get_clients(
    State(client_service): State<ClientService>,
) -> Json<ClientsResponse> {
    let clients = client_service.connected_clients().await;
    Json(clients)
}
