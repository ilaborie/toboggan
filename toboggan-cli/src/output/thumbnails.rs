//! Per-slide thumbnail generation and a self-contained slide-overview page.
//!
//! Thumbnails are rendered one Typst document per slide over the FULL slide list
//! (no render-target filtering), so page N corresponds exactly to slide N and the
//! overview's click-to-run targets stay aligned with `/api/slides`.

use std::path::Path;
use std::process::Command;

use serde::Serialize;
use toboggan_core::{Content, RenderTarget, Slide, SlideId, SlideKind, Talk};

use crate::error::{Result, TobogganCliError};
use crate::mermaid::MermaidRenderer;

/// Options controlling thumbnail generation.
#[derive(Debug, Clone)]
pub struct ThumbnailOptions {
    /// Whether to emit `search-index.json` and a search box in the overview.
    pub search: bool,
    /// Deck-level Mermaid settings, so a thumbnail draws its diagrams the same
    /// way the web client does.
    pub mermaid: MermaidRenderer,
    /// Point the overview's cards at a sibling `index.html` export rather than
    /// at a running server's `/run`.
    ///
    /// The default suits the server, which serves the overview at `/overview/`
    /// alongside the live deck. A published static site has no `/run` at all, so
    /// every card there was a link to a 404.
    pub static_links: bool,
}

impl Default for ThumbnailOptions {
    fn default() -> Self {
        Self {
            search: true,
            mermaid: MermaidRenderer::default(),
            static_links: false,
        }
    }
}

impl ThumbnailOptions {
    /// The defaults, drawing Mermaid diagrams the way this deck asks for.
    #[must_use]
    pub fn new(mermaid: MermaidRenderer) -> Self {
        Self {
            mermaid,
            ..Self::default()
        }
    }
}

/// One entry in the slide search index / overview grid.
#[derive(Debug, Clone, Serialize)]
struct SlideEntry {
    index: usize,
    display_number: usize,
    title: String,
    part: Option<String>,
    text: String,
    hidden_in_web: bool,
    /// This slide's 1-based position in the HTML export, if it appears there.
    ///
    /// Not `index + 1`. Thumbnails are rendered over the full slide list so that
    /// thumbnail N is slide N, but the HTML exporter filters the deck for
    /// [`RenderTarget::Web`] first and numbers its `id="slide-N"` anchors over
    /// what survives — so one `hidden_in = ["web"]` slide shifts every anchor
    /// after it. `None` for a slide the export drops, which therefore has no
    /// anchor to link to.
    web_number: Option<usize>,
}

/// Renders one PNG per slide plus `overview.html` (and `search-index.json`) into
/// `out_dir`, drawing the thumbnails with Typst.
///
/// The two halves are also available separately — [`render_typst_thumbnails`]
/// and [`write_overview_page`] — because the pictures and the page around them
/// are no longer one decision: `toboggan-server` photographs the real deck in a
/// headless browser and then writes the same page around the result.
///
/// # Errors
/// Returns an error if the output directory cannot be created, the `typst`
/// binary is missing or fails, or the index/overview files cannot be written.
pub fn generate_thumbnails(talk: &Talk, out_dir: &Path, options: &ThumbnailOptions) -> Result<()> {
    render_typst_thumbnails(talk, out_dir, &options.mermaid)?;
    write_overview_page(talk, out_dir, options)
}

/// Renders one `thumb-NNNN.png` per slide with Typst, and nothing else.
///
/// A second rendering of the deck, and a lossy one: Typst has no HTML, so a
/// slide's `<style>`, its raw markup and its terminals do not survive the trip
/// (see `output/typst.rs`). Kept as the fallback for a machine with no browser
/// on it, where an approximate overview beats none.
///
/// # Errors
/// Returns an error if the output directory cannot be created, or the `typst`
/// binary is missing or fails.
pub fn render_typst_thumbnails(
    talk: &Talk,
    out_dir: &Path,
    mermaid: &MermaidRenderer,
) -> Result<()> {
    std::fs::create_dir_all(out_dir)
        .map_err(|source| TobogganCliError::create_file(out_dir.to_path_buf(), source))?;

    // Slides reference images relative to the deck root (`../public/logo.png`),
    // so typst has to be told where that is. Without it every such slide fails
    // with "path would escape the project root".
    let root = super::typst::deck_root(talk);
    let slides_dir = talk.source_dir.as_deref().map(Path::new);
    for (index, slide) in talk.slides.iter().enumerate() {
        let typst_source = super::typst::generate_thumbnail_typst(slide, mermaid)?;
        let png = out_dir.join(format!("thumb-{index:04}.png"));
        compile_first_page_png(&typst_source, &png, root.as_deref(), slides_dir)?;
    }
    Ok(())
}

/// Writes `overview.html` (and `search-index.json`) around thumbnails that are
/// already there.
///
/// Knows nothing about how the pictures were made — only that slide N is
/// `thumb-NNNN.png` — which is what lets the browser and Typst renderers share
/// one page.
///
/// # Errors
/// Returns an error if the output directory cannot be created or the
/// index/overview files cannot be written.
pub fn write_overview_page(talk: &Talk, out_dir: &Path, options: &ThumbnailOptions) -> Result<()> {
    std::fs::create_dir_all(out_dir)
        .map_err(|source| TobogganCliError::create_file(out_dir.to_path_buf(), source))?;

    let entries = build_entries(talk);

    if options.search {
        let json = serde_json::to_vec_pretty(&entries)?;
        let index_path = out_dir.join("search-index.json");
        std::fs::write(&index_path, json)
            .map_err(|source| TobogganCliError::write_file(index_path, source))?;
    }

    let overview = render_overview(talk, &entries, options.search, options.static_links);
    let overview_path = out_dir.join("overview.html");
    std::fs::write(&overview_path, overview)
        .map_err(|source| TobogganCliError::write_file(overview_path, source))?;

    Ok(())
}

/// Compiles a single-slide Typst document and copies its FIRST page to `png`.
///
/// A `{p}` output template is used so slides whose content overflows into extra
/// pages still compile; only page 1 (the slide itself) is kept, keeping the
/// thumbnail index aligned with the slide index.
fn compile_first_page_png(
    typst_source: &[u8],
    png: &Path,
    root: Option<&Path>,
    slides_dir: Option<&Path>,
) -> Result<()> {
    let dir = tempfile::tempdir()
        .map_err(|source| TobogganCliError::create_file(png.to_path_buf(), source))?;
    // Alongside the slides when we know where they are, so a slide's relative
    // `#image("../public/…")` resolves the same way it does on disk; a temp dir
    // otherwise (a talk deserialized from `.toml` references no local files).
    //
    // A *unique* name, and a handle that removes it on drop, rather than the
    // fixed `.toboggan-thumb.typ` this used to write. Two generations of the
    // same deck could overlap — the server used to allow exactly that — and they
    // then took turns writing and deleting one path: `typst` read a half-written
    // document (`unclosed raw text`) or found none at all (`input file not
    // found`). The service no longer runs two generations at once, but the
    // scratch file is in the *user's* directory and a crash or a Ctrl-C between
    // the write and the removal used to strand it there; `NamedTempFile` cleans
    // up on every path out of this function, including a panic.
    let scratch = slides_dir
        .map(|slides| {
            tempfile::Builder::new()
                .prefix(".toboggan-thumb-")
                .suffix(".typ")
                .tempfile_in(slides)
        })
        .transpose()
        .map_err(|source| TobogganCliError::create_file(png.to_path_buf(), source))?;
    let input = match &scratch {
        Some(file) => file.path().to_path_buf(),
        None => dir.path().join("slide.typ"),
    };
    std::fs::write(&input, typst_source)
        .map_err(|source| TobogganCliError::write_file(input.clone(), source))?;

    let pattern = dir.path().join("page-{p}.png");
    let mut command = Command::new("typst");
    command.arg("compile");
    if let Some(root) = root {
        command.arg("--root").arg(root);
    }
    let output = command
        .arg(&input)
        .arg(&pattern)
        .output()
        .map_err(|err| TobogganCliError::typst(&err))?;

    // Closed before the status is checked, not after, so the one case that would
    // leave a file behind is not the failed compile — precisely the case where
    // the user then finds an unexplained scratch file next to their slides.
    // Best-effort, but not silent: the other two typst call sites already log.
    if let Some(file) = scratch
        && let Err(err) = file.close()
    {
        tracing::debug!("could not remove {}: {err}", input.display());
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TobogganCliError::typst_failed(&format!(
            "{}: {}",
            output.status,
            stderr.trim()
        )));
    }
    let first_page = dir.path().join("page-1.png");
    std::fs::copy(&first_page, png)
        .map_err(|source| TobogganCliError::write_file(png.to_path_buf(), source))?;
    Ok(())
}

fn build_entries(talk: &Talk) -> Vec<SlideEntry> {
    let mut current_part: Option<String> = None;
    let mut entries = Vec::with_capacity(talk.slides.len());
    // Counts only what the HTML export keeps, mirroring the `enumerate` in
    // `html::generate_html` over the already-web-filtered slide list.
    let mut web_position = 0_usize;

    for (index, slide) in talk.slides.iter().enumerate() {
        if slide.kind == SlideKind::Part {
            current_part = content_text(&slide.title);
        }
        let hidden_in_web = slide.hidden_in.contains(&RenderTarget::Web);
        let web_number = if hidden_in_web {
            None
        } else {
            web_position += 1;
            Some(web_position)
        };
        entries.push(SlideEntry {
            index,
            display_number: SlideId::new(index).display_number(),
            title: content_text(&slide.title).unwrap_or_default(),
            part: if slide.kind == SlideKind::Part {
                None
            } else {
                current_part.clone()
            },
            text: slide_text(slide),
            hidden_in_web,
            web_number,
        });
    }
    entries
}

/// Best-effort plain text of a slide body for search/popover.
fn slide_text(slide: &Slide) -> String {
    match &slide.body {
        Content::Html { alt: Some(alt), .. } => alt.clone(),
        Content::Html { raw, .. } => toboggan_stats::extract_text_from_html(raw),
        Content::Text { text } => text.clone(),
        Content::Empty => String::new(),
    }
}

fn content_text(content: &Content) -> Option<String> {
    match content {
        Content::Empty => None,
        other => Some(other.to_string()),
    }
}

fn render_overview(
    talk: &Talk,
    entries: &[SlideEntry],
    search: bool,
    static_links: bool,
) -> String {
    let title = escape(&talk.title);
    let lang = escape(talk.lang());

    // Emit a part divider whenever the part name changes, then the card.
    let mut last_part: Option<&str> = None;
    let mut blocks = Vec::with_capacity(entries.len());
    for entry in entries {
        let part = entry.part.as_deref();
        if part != last_part {
            if let Some(name) = part {
                blocks.push(format!(r#"    <div class="part">{}</div>"#, escape(name)));
            }
            last_part = part;
        }
        blocks.push(render_card(entry, static_links));
    }
    let cards = blocks.join("\n");

    let search_box = if search {
        r#"<input id="q" type="search" placeholder="Search slides…" autocomplete="off">"#
    } else {
        ""
    };

    let search_script = if search {
        r"<script>
  const q = document.getElementById('q');
  if (q) q.addEventListener('input', () => {
    const term = q.value.trim().toLowerCase();
    for (const card of document.querySelectorAll('.card')) {
      const hay = card.dataset.search || '';
      card.style.display = !term || hay.includes(term) ? '' : 'none';
    }
  });
</script>"
    } else {
        ""
    };

    // Carry a presenter token from this page's URL onto the slide links.
    //
    // Done here rather than when the links are written, because this page is
    // generated once and cached: baking the token in would write the secret to
    // disk and hand it to whoever opened the overview next. Reading it from the
    // address bar keeps it per-visitor, and a visitor without one is unaffected.
    let token_script = r"<script>
  const token = new URLSearchParams(location.search).get('token');
  if (token) for (const card of document.querySelectorAll('a.card')) {
    const url = new URL(card.href, location.href);
    url.searchParams.set('token', token);
    card.href = url.pathname + url.search + url.hash;
  }
</script>";

    format!(
        r#"<!doctype html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} — Slides</title>
<style>
  :root {{ color-scheme: dark; --accent: #4cc9f0; --bg: #0d1117; --fg: #e6edf5; --muted: #8b98a5; --card: #161b22; }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; font: 15px/1.5 system-ui, sans-serif; background: var(--bg); color: var(--fg); padding: 1.5rem; }}
  header {{ display: flex; gap: 1rem; align-items: center; flex-wrap: wrap; margin-bottom: 1.5rem; }}
  h1 {{ font-size: 1.4rem; margin: 0; }}
  input {{ flex: 1; min-width: 200px; padding: .6rem .9rem; border-radius: 8px; border: 1px solid #30363d; background: #0d1117; color: var(--fg); }}
  .grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 1rem; }}
  .part {{ grid-column: 1 / -1; margin: 1rem 0 .25rem; color: var(--accent); font-weight: 600; border-bottom: 1px solid #21262d; padding-bottom: .25rem; }}
  .card {{ background: var(--card); border: 1px solid #21262d; border-radius: 10px; overflow: hidden; text-decoration: none; color: inherit; transition: .15s; position: relative; }}
  .card:hover {{ border-color: var(--accent); transform: translateY(-2px); }}
  .card.unlinked {{ opacity: .55; }}
  .card.unlinked:hover {{ border-color: #21262d; transform: none; }}
  .card img {{ width: 100%; aspect-ratio: 16/9; object-fit: contain; background: #fff; display: block; }}
  .card .meta {{ padding: .5rem .7rem; }}
  .card .num {{ color: var(--muted); font-size: .8rem; }}
  .card .ttl {{ font-size: .9rem; }}
  .badge {{ position: absolute; top: .4rem; right: .4rem; background: #ff8c42; color: #04121a; font-size: .7rem; padding: .1rem .4rem; border-radius: 4px; }}
</style>
</head>
<body>
  <header>
    <h1>🛝 {title}</h1>
    {search_box}
  </header>
  <div class="grid">
{cards}
  </div>
{search_script}{token_script}
</body>
</html>"#,
    )
}

fn render_card(entry: &SlideEntry, static_links: bool) -> String {
    let badge = if entry.hidden_in_web {
        r#"<span class="badge">hidden on web</span>"#
    } else {
        ""
    };
    let haystack = escape(&format!(
        "{} {} {}",
        entry.title,
        entry.part.clone().unwrap_or_default(),
        entry.text
    ))
    .to_lowercase();

    let inner = format!(
        r#"      {badge}
      <img src="thumb-{index:04}.png" loading="lazy" alt="slide {num}">
      <div class="meta"><div class="num">{num}</div><div class="ttl">{title}</div></div>"#,
        index = entry.index,
        num = entry.display_number,
        title = escape(&entry.title),
    );
    let attrs = format!(
        r#"data-search="{haystack}" title="{tooltip}""#,
        tooltip = escape(&truncate(&entry.text, 200)),
    );

    match href(entry, static_links) {
        Some(href) => format!(
            r#"    <a class="card" href="{href}" {attrs}>
{inner}
    </a>"#
        ),
        // A web-hidden slide has no anchor in the export to point at, so the
        // card is not a link at all. `unlinked` drops the hover affordance that
        // would otherwise promise a click that does nothing; the badge above
        // already says why.
        None => format!(
            r#"    <div class="card unlinked" {attrs}>
{inner}
    </div>"#
        ),
    }
}

/// Where a card points.
///
/// `../index.html#slide-N` for a static site, because the action writes the deck
/// to `dist/index.html` and the overview to `dist/overview/`, so the export is
/// the overview's parent. The exporter emits those ids and its inlined navigator
/// honours the fragment, with or without scripting.
fn href(entry: &SlideEntry, static_links: bool) -> Option<String> {
    if static_links {
        entry
            .web_number
            .map(|number| format!("../index.html#slide-{number}"))
    } else {
        Some(format!("/run?slide={}", entry.index))
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        let mut out = text.chars().take(max).collect::<String>();
        out.push('…');
        out
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::BTreeSet;

    use toboggan_core::SlideKind;

    use super::*;

    /// Three slides, the middle one dropped from the web export.
    fn talk() -> Talk {
        let mut talk = Talk::new("Overview Test");
        talk.slides.push(Slide::new("Opening"));
        talk.slides.push(Slide {
            kind: SlideKind::Standard,
            title: Content::text("Handout only"),
            hidden_in: BTreeSet::from([RenderTarget::Web]),
            ..Default::default()
        });
        talk.slides.push(Slide::new("Closing"));
        talk
    }

    /// The numbering that made the static links worth having: a web-hidden
    /// slide takes no number, and everything after it moves up one — so the
    /// last slide is `#slide-2`, not `#slide-3`.
    #[test]
    fn a_web_hidden_slide_shifts_the_numbers_after_it() {
        let entries = build_entries(&talk());
        let numbers = entries
            .iter()
            .map(|entry| entry.web_number)
            .collect::<Vec<_>>();
        assert_eq!(numbers, vec![Some(1), None, Some(2)]);
    }

    /// The guard that actually matters: `web_number` is only useful if it names
    /// an anchor the exporter really emits, and the two are computed in
    /// different modules over differently filtered slide lists.
    #[test]
    fn every_web_number_names_an_anchor_the_html_export_emits() {
        let talk = talk();
        let filtered = super::super::filter_for(&talk, RenderTarget::Web);
        let html = super::super::html::generate_html(&filtered, None, "").expect("render html");
        let html = String::from_utf8(html).expect("utf-8");

        for entry in build_entries(&talk) {
            match entry.web_number {
                Some(number) => assert!(
                    html.contains(&format!(r#"id="slide-{number}""#)),
                    "slide {} claims #slide-{number}, which the export does not have",
                    entry.index
                ),
                // Nothing to point at, which is the whole reason for the `None`.
                None => assert!(!html.contains(&escape(&entry.title))),
            }
        }
        assert_eq!(html.matches(r#"class="toboggan-slide""#).count(), 2);
    }

    /// A published site has no `/run`, so the cards address the export beside it.
    #[test]
    fn static_links_point_at_the_sibling_export() {
        let entries = build_entries(&talk());
        let overview = render_overview(&talk(), &entries, false, true);

        assert!(overview.contains(r#"href="../index.html#slide-1""#));
        assert!(overview.contains(r#"href="../index.html#slide-2""#));
        assert!(!overview.contains("/run?slide="));
        // The web-hidden slide is a card, but not a link.
        assert!(overview.contains(r#"<div class="card unlinked""#));
        assert!(!overview.contains("#slide-3"));
    }

    /// The server keeps what it had: the overview at `/overview/` sits beside a
    /// live `/run`, and its links are 0-based over the unfiltered deck.
    #[test]
    fn the_default_still_links_to_the_running_server() {
        let entries = build_entries(&talk());
        let overview = render_overview(&talk(), &entries, false, false);

        for index in 0..3 {
            assert!(overview.contains(&format!(r#"href="/run?slide={index}""#)));
        }
        assert!(!overview.contains("index.html#"));
        // The stylesheet always carries the `.card.unlinked` rules; what must
        // not appear is a card wearing the class.
        assert!(!overview.contains(r#"class="card unlinked""#));
    }
}
