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
}

impl Default for ThumbnailOptions {
    fn default() -> Self {
        Self {
            search: true,
            mermaid: MermaidRenderer::default(),
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
}

/// Renders one PNG per slide plus `overview.html` (and `search-index.json`) into
/// `out_dir`.
///
/// # Errors
/// Returns an error if the output directory cannot be created, the `typst`
/// binary is missing or fails, or the index/overview files cannot be written.
pub fn generate_thumbnails(talk: &Talk, out_dir: &Path, options: &ThumbnailOptions) -> Result<()> {
    std::fs::create_dir_all(out_dir)
        .map_err(|source| TobogganCliError::create_file(out_dir.to_path_buf(), source))?;

    // Slides reference images relative to the deck root (`../public/logo.png`),
    // so typst has to be told where that is. Without it every such slide fails
    // with "path would escape the project root".
    let root = super::typst::deck_root(talk);
    let slides_dir = talk.source_dir.as_deref().map(Path::new);
    for (index, slide) in talk.slides.iter().enumerate() {
        let typst_source = super::typst::generate_thumbnail_typst(slide, &options.mermaid)?;
        let png = out_dir.join(format!("thumb-{index:04}.png"));
        compile_first_page_png(&typst_source, &png, root.as_deref(), slides_dir)?;
    }

    let entries = build_entries(talk);

    if options.search {
        let json = serde_json::to_vec_pretty(&entries)?;
        let index_path = out_dir.join("search-index.json");
        std::fs::write(&index_path, json)
            .map_err(|source| TobogganCliError::write_file(index_path, source))?;
    }

    let overview = render_overview(talk, &entries, options.search);
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
    let input = slides_dir.map_or_else(
        || dir.path().join("slide.typ"),
        |slides| slides.join(".toboggan-thumb.typ"),
    );
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

    // Removed before the status is checked, not after. This scratch file is
    // written into the user's own slides folder, and the cleanup used to sit
    // below the early return — so the one case that leaves it behind was a
    // failed compile, which is precisely the case where the user then finds an
    // unexplained `.toboggan-thumb.typ` next to their slides.
    //
    // Best-effort, but not silent: the other two typst call sites already log.
    if slides_dir.is_some()
        && let Err(err) = std::fs::remove_file(&input)
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

    for (index, slide) in talk.slides.iter().enumerate() {
        if slide.kind == SlideKind::Part {
            current_part = content_text(&slide.title);
        }
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
            hidden_in_web: slide.hidden_in.contains(&RenderTarget::Web),
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

fn render_overview(talk: &Talk, entries: &[SlideEntry], search: bool) -> String {
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
        blocks.push(render_card(entry));
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
  if (token) for (const card of document.querySelectorAll('.card')) {
    const url = new URL(card.href, location.href);
    url.searchParams.set('token', token);
    card.href = url.pathname + url.search;
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

fn render_card(entry: &SlideEntry) -> String {
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

    format!(
        r#"    <a class="card" href="/run?slide={index}" data-search="{haystack}" title="{tooltip}">
      {badge}
      <img src="thumb-{index:04}.png" loading="lazy" alt="slide {num}">
      <div class="meta"><div class="num">{num}</div><div class="ttl">{title}</div></div>
    </a>"#,
        index = entry.index,
        num = entry.display_number,
        title = escape(&entry.title),
        tooltip = escape(&truncate(&entry.text, 200)),
    )
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
