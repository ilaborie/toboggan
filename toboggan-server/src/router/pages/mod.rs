//! Server-rendered HTML pages: the landing page (`/`), the run/present app
//! (`/run`), and the packaged guide (`/guide`). The slide-overview page
//! (`/slides`) lives in [`overview`].

pub(super) mod overview;
pub(super) mod pdf;

use std::sync::OnceLock;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use toboggan_cli::OutputFormat;
use toboggan_core::{SlideKind, Talk};
use tracing::error;

use crate::services::TalkService;

/// Landing page at `/`.
pub(super) async fn homepage(State(talk_service): State<TalkService>) -> Html<String> {
    let talk = talk_service.talk().await;
    Html(render_homepage(&talk))
}

/// Serves the embedded present/run single-page app at `/run`.
pub(super) async fn run_app() -> Response {
    embedded_page("index.html")
}

/// Serves the presenter view at `/presenter`.
///
/// A second page of the same application, not a second client: it opens its own
/// socket, follows the same broadcast state and drives the deck with the same
/// keys. Two windows, one talk — which is the point, since the presenter's
/// screen and the projector are two different screens.
pub(super) async fn presenter_app() -> Response {
    embedded_page("presenter.html")
}

fn embedded_page(name: &'static str) -> Response {
    match super::static_assets::WebAppAssets::get(name) {
        Some(content) => super::static_assets::asset_response(name, content.data.into_owned()),
        None => (StatusCode::NOT_FOUND, "web app not built").into_response(),
    }
}

/// Serves the packaged user guide at `/guide`, rendered once and cached.
///
/// A render failure is logged and answered `500` rather than cached as a
/// success: caching it pinned the error page for the process lifetime, and
/// returning `200` hid the failure from both the operator's logs and any HTTP
/// monitoring.
pub(super) async fn guide() -> Response {
    static GUIDE_HTML: OnceLock<Result<String, String>> = OnceLock::new();
    match GUIDE_HTML.get_or_init(|| render_guide().map_err(|err| err.to_string())) {
        // `&'static str` from the `OnceLock`, so serving it costs no copy.
        Ok(html) => Html(html.as_str()).into_response(),
        Err(err) => {
            error!("guide render failed: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!("<h1>Guide unavailable</h1><p>{err}</p>")),
            )
                .into_response()
        }
    }
}

/// Serves the guide's bundled `public/` assets (CSS, fonts, images) at
/// `/guide/public/{path}`.
pub(super) async fn guide_asset(Path(path): Path<String>) -> Response {
    match super::static_assets::GuideAssets::get(&path) {
        Some(content) => super::static_assets::asset_response(&path, content.data.into_owned()),
        None => (StatusCode::NOT_FOUND, "guide asset not found").into_response(),
    }
}

const GUIDE_TOML: &str = include_str!("../../../../examples/toboggan-guide/toboggan-guide.toml");

fn render_guide() -> anyhow::Result<String> {
    let talk = toml::from_str::<Talk>(GUIDE_TOML)?;
    // The guide's own assets are served at `/guide/public/`, not `/public/`, so
    // that is the base its export is rendered against. This used to be a pair of
    // string replacements here; the renderer does it now, and does it for every
    // spelling of the URL rather than the one the guide happens to use.
    let bytes = toboggan_cli::output::serialize_talk(&talk, OutputFormat::Html, "/guide/")
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    Ok(String::from_utf8(bytes)?)
}

fn render_homepage(talk: &Talk) -> String {
    let title = escape(&talk.title);
    let lang = escape_attribute(talk.lang());
    let date = talk.date.to_string();
    let total = talk.slides.len();
    let parts = talk
        .slides
        .iter()
        .filter(|slide| slide.kind == SlideKind::Part)
        .count();
    let content_slides = talk
        .slides
        .iter()
        .filter(|slide| slide.kind != SlideKind::Part)
        .count();

    format!(
        r#"<!doctype html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} — Toboggan</title>
<style>
  :root {{ color-scheme: dark; --accent: #4cc9f0; --accent2: #ff8c42; --bg: #0d1117; --fg: #e6edf5; --muted: #8b98a5; --card: #161b22; }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; min-height: 100vh; font: 16px/1.5 system-ui, sans-serif; color: var(--fg);
    background: var(--bg) radial-gradient(60vw 60vw at 80% 10%, rgba(76,201,240,.12), transparent 60%)
      radial-gradient(50vw 50vw at 10% 90%, rgba(255,140,66,.10), transparent 60%);
    display: grid; place-items: center; padding: 2rem; }}
  .card {{ background: var(--card); border: 1px solid #21262d; border-radius: 16px; padding: 2.5rem;
    max-width: 640px; width: 100%; box-shadow: 0 1rem 3rem rgba(0,0,0,.4); }}
  .sled {{ font-size: 3rem; }}
  h1 {{ font-size: 2.2rem; margin: .2em 0 0;
    background: linear-gradient(100deg, var(--accent), var(--accent2));
    -webkit-background-clip: text; background-clip: text; color: transparent; }}
  .date {{ color: var(--muted); margin: .2em 0 1.5em; }}
  .stats {{ display: flex; gap: 1.5rem; margin-bottom: 1.5rem; color: var(--muted); font-size: .9rem; }}
  .stats b {{ color: var(--fg); font-size: 1.4rem; display: block; }}
  .links {{ display: grid; grid-template-columns: 1fr 1fr; gap: .75rem; }}
  a.btn {{ display: block; padding: .8rem 1rem; border-radius: 10px; text-decoration: none;
    background: #21262d; color: var(--fg); border: 1px solid #30363d; transition: .15s; }}
  a.btn:hover {{ border-color: var(--accent); transform: translateY(-1px); }}
  a.primary {{ background: linear-gradient(100deg, var(--accent), var(--accent2)); color: #04121a; font-weight: 600; grid-column: 1 / -1; }}
</style>
</head>
<body>
  <main class="card">
    <div class="sled">🛝</div>
    <h1>{title}</h1>
    <p class="date">{date}</p>
    <div class="stats">
      <div><b>{total}</b> slides</div>
      <div><b>{content_slides}</b> content</div>
      <div><b>{parts}</b> parts</div>
    </div>
    <nav class="links">
      <a class="btn primary" href="/run">▶ Run the presentation</a>
      <a class="btn" href="/presenter">🎙 Presenter view</a>
      <a class="btn" href="/slides">🗂 Slide overview</a>
      <a class="btn" href="/guide">📖 User guide</a>
      <a class="btn" href="/download.pdf">⬇ Download PDF</a>
      <a class="btn" href="/doc">🔌 API docs</a>
    </nav>
  </main>
</body>
</html>"#,
    )
}

/// Escapes the three characters that would break out of HTML text content.
///
/// Shared with [`overview`], which renders the same kind of server-side error
/// page. **Text content only** — an attribute value needs
/// [`escape_attribute`], which also handles the quote that would end it.
pub(super) fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escapes text for a double-quoted HTML attribute value.
///
/// [`escape`] leaves `"` alone, which is correct between tags and wrong inside
/// them: the deck's `lang` is author-supplied and lands in `<html lang="…">`, so
/// a quote in it used to close the attribute and open whatever followed as
/// markup. Both escapers in `toboggan-cli` already handled this; the server's
/// did not.
pub(super) fn escape_attribute(text: &str) -> String {
    escape(text).replace('"', "&quot;")
}
