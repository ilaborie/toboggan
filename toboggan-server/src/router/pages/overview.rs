//! The slide-overview page (`/slides`) and its generated assets (`/overview/*`).
//!
//! Thumbnails are generated lazily on the first `/slides` hit (see
//! [`crate::services::ThumbnailService`]). While generation is in flight the page
//! auto-refreshes; if `typst` is unavailable it shows a clean explanation instead
//! of a server error.

use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse, Redirect, Response};

use crate::TobogganState;
use crate::services::ThumbStatus;

/// Landing point for the slide overview at `/slides`.
pub(crate) async fn slides_page(State(state): State<TobogganState>) -> Response {
    match state.ensure_thumbnails().await {
        // Redirect so the overview's relative `thumb-NNNN.png` URLs resolve under
        // `/overview/`.
        ThumbStatus::Ready => Redirect::to("/overview/overview.html").into_response(),
        ThumbStatus::Pending => Html(GENERATING_PAGE).into_response(),
        ThumbStatus::Unavailable(reason) => Html(unavailable_page(&reason)).into_response(),
    }
}

/// Serves a generated overview asset (`overview.html`, `thumb-*.png`, the search
/// index) from the thumbnail cache.
pub(crate) async fn overview_asset(
    State(state): State<TobogganState>,
    Path(rel): Path<String>,
) -> Response {
    match state.thumbnail_asset(&rel).await {
        Some(bytes) => super::super::static_assets::asset_response(&rel, bytes),
        None => Redirect::to("/slides").into_response(),
    }
}

const GENERATING_PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="refresh" content="2">
<title>Generating slide overview…</title>
<style>
  :root { color-scheme: dark; }
  body { margin: 0; min-height: 100vh; display: grid; place-items: center;
    font: 16px/1.5 system-ui, sans-serif; background: #0d1117; color: #e6edf5; }
  .box { text-align: center; }
  .sled { font-size: 3rem; animation: slide 1.2s ease-in-out infinite alternate; }
  @keyframes slide { from { transform: translateX(-12px); } to { transform: translateX(12px); } }
  .muted { color: #8b98a5; }
</style>
</head>
<body>
  <div class="box">
    <div class="sled">🛝</div>
    <h1>Rendering slide thumbnails…</h1>
    <p class="muted">This page refreshes automatically.</p>
  </div>
</body>
</html>"#;

fn unavailable_page(reason: &str) -> String {
    let reason = reason
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Slide overview unavailable</title>
<style>
  :root {{ color-scheme: dark; }}
  body {{ margin: 0; min-height: 100vh; display: grid; place-items: center;
    font: 16px/1.5 system-ui, sans-serif; background: #0d1117; color: #e6edf5; padding: 2rem; }}
  .box {{ max-width: 560px; text-align: center; }}
  .muted {{ color: #8b98a5; }}
  code {{ background: #161b22; padding: .1rem .4rem; border-radius: 4px; }}
  a {{ color: #4cc9f0; }}
</style>
</head>
<body>
  <div class="box">
    <div style="font-size:3rem">🛝</div>
    <h1>Slide overview unavailable</h1>
    <p class="muted">{reason}</p>
    <p>Install the <a href="https://typst.app/">typst</a> binary, then reload. The
       presentation itself is unaffected — <a href="/run">run it</a> or return to the
       <a href="/">homepage</a>.</p>
  </div>
</body>
</html>"#,
    )
}
