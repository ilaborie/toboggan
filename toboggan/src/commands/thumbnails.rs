use std::path::{Path, PathBuf};

use toboggan_server::{OverviewOptions, ResolvedRenderer, ThumbnailRenderer, generate_overview};

/// What the caller asked for, beyond the deck itself.
pub(crate) struct Options {
    pub(crate) output: Option<PathBuf>,
    pub(crate) search: bool,
    pub(crate) static_links: bool,
    pub(crate) renderer: Option<ThumbnailRenderer>,
    pub(crate) browser: Option<PathBuf>,
}

/// Generates per-slide thumbnails, a search index, and a self-contained overview
/// page for a presentation folder.
///
/// The thumbnails are photographs of the real deck, taken by a headless browser
/// against a private server started for the purpose — so the overview and the
/// projector show the same rendering. Where no browser can be found, `auto`
/// falls back to redrawing the deck with `typst`, which cannot render a deck's
/// HTML, CSS or terminals.
///
/// # Errors
/// Returns an error if the folder cannot be parsed, or the thumbnails cannot be
/// rendered (no browser *and* no `typst`, or either of them failing).
#[allow(clippy::print_stdout, clippy::print_stderr)]
pub(crate) async fn generate(
    input: &Path,
    mut settings: toboggan_cli::Settings,
    options: Options,
) -> anyhow::Result<()> {
    // Decided before the deck is parsed, so "install typst" is not preceded by
    // ten seconds of work that is about to be thrown away. Only the Typst path
    // needs the binary; the browser path is checked when the browser is
    // launched.
    let renderer = options.renderer.unwrap_or_default();
    let resolved = renderer.resolve(options.browser.as_deref()).await?;
    if matches!(
        resolved,
        ResolvedRenderer::Typst | ResolvedRenderer::TypstInstead(_)
    ) {
        super::ensure_typst()?;
    }

    let deck = super::deck::resolve_deck(input);
    let slides = deck.slides;
    settings.input = Some(slides.clone());
    let talk = super::deck::build_talk(&slides, &settings)?;
    let mermaid = toboggan_cli::mermaid_renderer(&settings)?;

    let out_dir = options.output.unwrap_or_else(|| PathBuf::from("overview"));
    let overview = OverviewOptions {
        // The mode is passed through unchanged — it is what decides whether a
        // browser that will not launch may fall back, and `generate_overview`
        // owns that. What the decision above saves is the *search*: naming the
        // browser it found leaves that call a single `--version` instead of
        // another sweep of every candidate.
        renderer,
        browser: match resolved {
            ResolvedRenderer::Photograph(browser) => Some(browser),
            ResolvedRenderer::Typst | ResolvedRenderer::TypstInstead(_) => options.browser,
        },
        // The deck's own pictures: without this the private server has no
        // `/public/`, and every `<img>` is photographed as a broken image.
        public_dir: deck.public,
        search: options.search,
        static_links: options.static_links,
    };
    let drawn = generate_overview(&talk, &mermaid, &out_dir, &overview).await?;

    // Printed, not logged. `warn!` goes to an env filter that defaults to
    // `ERROR`, so this said nothing at all unless the user had thought to set
    // `RUST_LOG` — and the line below reads the same either way, which left a
    // deck that lost every `<style>`, raw HTML block and terminal announcing
    // itself as a clean success.
    if let Some(warning) = drawn.warning() {
        eprintln!("⚠️  {warning}");
    }

    println!(
        "✅ Wrote {} thumbnails + overview.html to {} ({})",
        talk.slides.len(),
        out_dir.display(),
        drawn.describe(),
    );
    Ok(())
}
