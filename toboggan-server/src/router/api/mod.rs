use std::time::Instant;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use toboggan_core::{
    ClientsResponse, Command, Notification, OutlineResponse, SlideId, SlidesResponse, TalkResponse,
};
use tracing::{info, warn};

use super::presenter::Presenter;
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

    // Step counts are computed when the deck loads, not per request: deriving
    // them here meant several HTML parses per slide on every call. They are
    // attached through `with_step_counts`, which refuses a set that does not
    // line up with the titles it will be read against.
    let mut result = TalkResponse::from(talk.as_ref())
        .with_step_counts(talk_service.step_counts().await.to_vec());

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

/// The deck as a searchable list, for the presenter's slide picker.
///
/// Separate from `GET /api/talk` rather than folded into it: this is a slide's
/// whole body and its notes as plain text, which is most of the deck again, and
/// every client that draws a progress bar asks for the talk.
pub(super) async fn get_outline(State(talk_service): State<TalkService>) -> impl IntoResponse {
    let slides = talk_service.outline().await.to_vec();

    Json(OutlineResponse { slides })
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

/// Drives the deck over plain HTTP.
///
/// `Presenter` first, and before `Json`: the gate has to run before the body is
/// read, so a refused caller is told `403` rather than having its command
/// parsed and validated first.
pub(super) async fn post_command(
    _: Presenter,
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

/// Lists who is connected — names, roles and **IP addresses**.
///
/// Gated for that last one: the audience has no business enumerating the rest
/// of the audience, and this is an operator's view of the room rather than a
/// part of the presentation.
pub(super) async fn get_clients(
    _: Presenter,
    State(client_service): State<ClientService>,
) -> Json<ClientsResponse> {
    let clients = client_service.connected_clients().await;
    Json(clients)
}
