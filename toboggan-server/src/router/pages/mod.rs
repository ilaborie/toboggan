//! Server-rendered HTML pages: the landing page (`/`), the run/present app
//! (`/run`), and the packaged guide (`/guide`). The slide-overview page
//! (`/slides`) lives in [`overview`].

pub(super) mod overview;
pub(super) mod pdf;

use std::sync::OnceLock;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use qrcode::QrCode;
use qrcode::render::svg;
use toboggan_cli::OutputFormat;
use toboggan_core::{SlideKind, Talk};
use tracing::error;

use crate::services::TalkService;

/// Landing page at `/`.
///
/// Takes the request's own query string so the links below can carry the token
/// it was opened with. Without that, a remote presenter who opened `/?token=…`
/// was demoted to audience by clicking "Run the presentation" — and, before the
/// presenter view could show a toast, told nothing about it.
pub(super) async fn homepage(
    State(talk_service): State<TalkService>,
    headers: HeaderMap,
    uri: Uri,
) -> Html<String> {
    let talk = talk_service.talk().await;
    let query = carried_query(&uri);
    let phone_link = phone_link(&headers, &query);
    Html(render_homepage(&talk, &query, phone_link.as_deref()))
}

/// The URL a phone should be pointed at, as a QR code's payload.
///
/// Built from the `Host` header rather than from the bind address on purpose.
/// A wildcard bind (`--host 0.0.0.0`) has no single address to name — which is
/// why the startup log prints `<this-machine>` and leaves it to the operator —
/// but the browser reading this page necessarily reached the server *somehow*,
/// and that authority is reachable by construction. The token rides along in
/// `query`, so a presenter who opened this page with one hands the phone the
/// whole configuration in a single scan.
fn phone_link(headers: &HeaderMap, query: &str) -> Option<String> {
    let host = headers.get(axum::http::header::HOST)?.to_str().ok()?;
    if host.is_empty() {
        return None;
    }
    // The client wants the origin; it appends `/api/…` itself.
    Some(format!("http://{host}/{query}"))
}

/// Renders `link` as an inline SVG QR code.
///
/// Inline because the alternative is another route to serve it from, and this
/// page is already server-rendered.
fn qr_svg(link: &str) -> Option<String> {
    let code = QrCode::new(link.as_bytes()).ok()?;
    Some(
        code.render()
            .min_dimensions(180, 180)
            .dark_color(svg::Color("#1b1b1f"))
            .light_color(svg::Color("#ffffff"))
            .build(),
    )
}

/// The part of this request's query string worth passing to the next page.
///
/// Only the token: everything else on a page URL is that page's own business.
fn carried_query(uri: &Uri) -> String {
    uri.query()
        .into_iter()
        .flat_map(|query| query.split('&'))
        .find(|pair| pair.starts_with("token="))
        .map(|pair| format!("?{pair}"))
        .unwrap_or_default()
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
    let bytes = toboggan_cli::output::serialize_talk(
        &talk,
        OutputFormat::Html,
        "/guide/",
        // The guide's diagrams were already drawn into its HTML when the
        // checked-in artifact was built, and the HTML path never re-renders
        // one — so the renderer handed over here is never consulted.
        &toboggan_cli::mermaid::MermaidRenderer::default(),
    )
    .map_err(|err| anyhow::anyhow!("{err}"))?;
    Ok(String::from_utf8(bytes)?)
}

fn render_homepage(talk: &Talk, query: &str, phone_link: Option<&str>) -> String {
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

    // Rendered only when the request carried a `Host` and the payload encodes:
    // a page with no QR is better than one showing a code that goes nowhere.
    let phone_section = phone_link
        .and_then(|link| Some((link, qr_svg(link)?)))
        .map(|(link, svg)| {
            format!(
                r#"<section class="phone">
      <div class="qr">{svg}</div>
      <p>Scan with the Toboggan app to drive the deck from your phone.<br>
      <code>{link}</code></p>
    </section>"#,
                link = escape(link),
            )
        })
        .unwrap_or_default();

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
  .phone {{ display: flex; align-items: center; gap: 1rem; margin-bottom: 1.5rem;
            padding: 1rem; border-radius: .75rem; background: rgba(127,127,127,.08); }}
  /* The code has to stay a fixed, generous size: a QR scaled down by a flex
     row is a QR a phone cannot read. */
  .phone .qr {{ flex: 0 0 auto; width: 132px; height: 132px; background: #fff;
                padding: .4rem; border-radius: .5rem; }}
  .phone .qr svg {{ display: block; width: 100%; height: 100%; }}
  .phone p {{ margin: 0; color: var(--muted); font-size: .85rem; line-height: 1.5; }}
  .phone code {{ font-size: .8rem; word-break: break-all; }}
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
    {phone_section}
    <nav class="links">
      <a class="btn primary" href="/run{query}">▶ Run the presentation</a>
      <a class="btn" href="/presenter{query}">🎙 Presenter view</a>
      <a class="btn" href="/slides{query}">🗂 Slide overview</a>
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
