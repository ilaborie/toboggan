use axum::body::Bytes;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

/// Embedded web assets from toboggan-web/dist
#[derive(RustEmbed)]
#[folder = "../toboggan-web/dist"]
pub(super) struct WebAppAssets;

/// Embedded `public/` assets (CSS, fonts, images) for the packaged guide,
/// served at `/guide/public/`.
#[derive(RustEmbed)]
#[folder = "../examples/toboggan-guide/public"]
pub(super) struct GuideAssets;

/// Builds a `200 OK` asset response, guessing the content type from `path`.
/// Shared by every handler that serves bytes (embedded web/guide assets and the
/// generated slide-overview cache).
pub(super) fn asset_response(path: &str, bytes: impl Into<Bytes>) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, mime.as_ref().to_owned())],
        bytes.into(),
    )
        .into_response()
}
