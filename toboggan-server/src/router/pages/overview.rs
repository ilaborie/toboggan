//! The slide-overview page (`/slides`) and its generated assets (`/overview/*`).
//!
//! Thumbnails are photographed as the server starts (see
//! [`crate::services::ThumbnailService`]), and on the first request that wants
//! them when `--no-eager-thumbnails` says not to. While generation is in flight the page
//! auto-refreshes; when neither renderer can draw them — no browser *and* no
//! `typst`, or one of them failing — it shows a clean explanation instead of a
//! server error.

use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};

use crate::TobogganState;
use crate::services::{AssetLookup, ThumbStatus};

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

/// Serves one presented slide's thumbnail, for the presenter view's slide picker.
///
/// Addressed by *presented* index — the same number `Command::GoTo` and
/// `/api/slides/{index}` take — while the thumbnails on disk are named over the
/// deck as authored. [`TobogganState::presented_thumbnail`] crosses between the
/// two, so the client never has to know the deck hides anything.
///
/// Answers `503` while the thumbnails are still being made, rather than the
/// `303` the asset route sends: the caller here is an `<img>`, and a redirect
/// to `/slides` hands it an HTML page, which renders as a broken image that
/// never recovers. A `503` with `Retry-After` says "ask again" in the one way
/// both a browser and a script can act on.
pub(crate) async fn presented_thumbnail(
    State(state): State<TobogganState>,
    Path(index): Path<usize>,
) -> Response {
    match state.presented_thumbnail(index).await {
        AssetLookup::Found(bytes) => (
            [
                (header::CONTENT_TYPE, "image/png"),
                // The deck reloads under the speaker, and the thumbnail of slide
                // 4 is a different picture afterwards at the same URL.
                (header::CACHE_CONTROL, "no-cache"),
            ],
            bytes,
        )
            .into_response(),
        AssetLookup::NotReady => (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::RETRY_AFTER, "1")],
            "slide thumbnails are still being generated",
        )
            .into_response(),
        AssetLookup::Missing => (
            StatusCode::NOT_FOUND,
            format!("no thumbnail for slide {index}"),
        )
            .into_response(),
    }
}

/// Serves a generated overview asset (`overview.html`, `thumb-*.png`, the search
/// index) from the thumbnail cache.
pub(crate) async fn overview_asset(
    State(state): State<TobogganState>,
    Path(rel): Path<String>,
) -> Response {
    match state.thumbnail_asset(&rel).await {
        AssetLookup::Found(bytes) => super::super::static_assets::asset_response(&rel, bytes),
        // Regenerating after a reload: `/slides` serves the "generating…" page,
        // which retries until the overview is back.
        AssetLookup::NotReady => Redirect::to("/slides").into_response(),
        // Ready, but no such asset. `/slides` redirects straight back here when
        // the overview is ready, so bouncing this case would loop until the
        // browser gives up.
        AssetLookup::Missing => {
            (StatusCode::NOT_FOUND, format!("no such asset: {rel}")).into_response()
        }
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
    let reason = super::escape(reason);
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
    <p>Thumbnails are photographed in a headless browser, and redrawn with
       <a href="https://typst.app/">typst</a> where none can be found. Install
       Chrome, Chromium or Edge — or <code>typst</code> — then restart
       <code>toboggan</code>, or save a slide if it is watching the folder.
       Reloading this page alone will not retry.</p>
    <p>The presentation itself is unaffected — <a href="/run">run it</a> or return
       to the <a href="/">homepage</a>.</p>
  </div>
</body>
</html>"#,
    )
}
