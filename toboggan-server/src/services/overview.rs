//! Producing a slide-overview directory: the thumbnails, and the page around
//! them.
//!
//! One entry point for the two callers that want one — the lazily-generated
//! overview behind `/slides`, and `toboggan thumbnails` — so the choice of
//! renderer is made in one place and behaves the same from both.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use toboggan_cli::mermaid::MermaidRenderer;
use toboggan_cli::output::{ThumbnailOptions, render_typst_thumbnails, write_overview_page};
use toboggan_core::Talk;
use tracing::info;

use super::shots::{ShotFailure, ShotOptions, find_browser, shoot_slides};

/// How the per-slide pictures are made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ThumbnailRenderer {
    /// Photograph the real deck if a browser can be found, and fall back to
    /// Typst if not.
    #[default]
    Auto,
    /// Photograph the real deck, and fail if no browser can be found.
    ///
    /// The honest choice for CI: `auto` silently produces a *different*,
    /// approximate overview when the browser is missing, and a published site
    /// that quietly changed how it looks is worse than a build that stopped.
    Browser,
    /// Redraw the deck as a Typst document.
    ///
    /// Needs the `typst` binary rather than a browser, and cannot render a
    /// deck's HTML, CSS or terminals.
    Typst,
}

/// Which renderer actually drew the overview.
///
/// Returned rather than only logged, because `auto` choosing Typst is a fact
/// the *person* needs and not telemetry: the two renderers produce visibly
/// different overviews, and a caller that reports success the same way either
/// time has told them nothing. `toboggan thumbnails` did exactly that — its
/// `warn!` went to an env filter that defaults to `ERROR`, so the only trace of
/// a deck losing every `<style>`, raw HTML block and terminal was a log line
/// nobody was shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drawn {
    /// Photographed in a headless browser.
    Photographed,
    /// Redrawn with Typst, which is what was asked for.
    Redrawn,
    /// Redrawn with Typst after a browser could not be used, carrying why.
    FellBackToTypst(String),
}

impl Drawn {
    /// A sentence naming what the deck lost, when it lost something.
    ///
    /// Phrased for a person rather than a log: whoever sees it is the one who
    /// can install a browser.
    #[must_use]
    pub fn warning(&self) -> Option<String> {
        match self {
            Self::Photographed | Self::Redrawn => None,
            Self::FellBackToTypst(reason) => Some(format!(
                "The slide overview fell back to Typst because {reason}. Typst cannot render a \
                 deck's HTML, CSS or terminals, so the thumbnails will not match the projector \
                 — install a browser, set CHROME, or pass --browser."
            )),
        }
    }

    /// How to describe the pictures in a line reporting success.
    #[must_use]
    pub fn describe(&self) -> &'static str {
        match self {
            Self::Photographed => "photographed",
            Self::Redrawn | Self::FellBackToTypst(_) => "redrawn with Typst",
        }
    }
}

/// Everything [`generate_overview`] needs beyond the talk itself.
#[derive(Debug, Clone)]
pub struct OverviewOptions {
    /// Which renderer draws the thumbnails.
    pub renderer: ThumbnailRenderer,
    /// An explicit browser binary for [`ThumbnailRenderer::Browser`].
    pub browser: Option<PathBuf>,
    /// The deck's `public/` directory, so a slide's pictures resolve while it
    /// is being photographed.
    pub public_dir: Option<PathBuf>,
    /// Whether to emit `search-index.json` and a search box.
    pub search: bool,
    /// Point the overview's cards at a sibling static export rather than at a
    /// running server's `/run`.
    pub static_links: bool,
}

impl Default for OverviewOptions {
    fn default() -> Self {
        Self {
            renderer: ThumbnailRenderer::default(),
            browser: None,
            public_dir: None,
            // Matches `ThumbnailOptions`: the search box is what makes an
            // overview of a sixty-slide deck usable.
            search: true,
            static_links: false,
        }
    }
}

/// Renders `thumb-NNNN.png` per slide plus `overview.html` into `out_dir`.
///
/// Returns which renderer drew the pictures — see [`Drawn`], which the caller
/// is expected to report rather than leave to the log.
///
/// # Errors
/// Returns an error if the thumbnails cannot be drawn — no browser and no
/// `typst`, or either of them failing — or the overview page cannot be written.
pub async fn generate_overview(
    talk: &Talk,
    mermaid: &MermaidRenderer,
    out_dir: &Path,
    options: &OverviewOptions,
) -> anyhow::Result<Drawn> {
    let drawn = render_thumbnails(talk, mermaid, out_dir, options).await?;

    let thumbnails = ThumbnailOptions {
        search: options.search,
        static_links: options.static_links,
        ..ThumbnailOptions::new(mermaid.clone())
    };
    let (talk, out_dir) = (talk.clone(), out_dir.to_path_buf());
    tokio::task::spawn_blocking(move || write_overview_page(&talk, &out_dir, &thumbnails))
        .await?
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    Ok(drawn)
}

async fn render_thumbnails(
    talk: &Talk,
    mermaid: &MermaidRenderer,
    out_dir: &Path,
    options: &OverviewOptions,
) -> anyhow::Result<Drawn> {
    let shots = ShotOptions {
        browser: options.browser.clone(),
        public_dir: options.public_dir.clone(),
    };

    match options.renderer {
        ThumbnailRenderer::Browser => {
            shoot_slides(talk, mermaid, out_dir, &shots).await?;
            Ok(Drawn::Photographed)
        }
        ThumbnailRenderer::Typst => {
            typst_thumbnails(talk, mermaid, out_dir).await?;
            Ok(Drawn::Redrawn)
        }
        ThumbnailRenderer::Auto => {
            let reason = match find_browser(options.browser.as_deref()) {
                None => "no Chrome, Chromium or Edge could be started".to_owned(),
                Some(browser) => {
                    info!(browser = %browser.display(), "Photographing the deck for the overview");
                    match shoot_slides(talk, mermaid, out_dir, &shots).await {
                        Ok(()) => return Ok(Drawn::Photographed),
                        // Detection said yes and the launch said no — a
                        // container without the shared libraries, most likely.
                        // Still a missing browser, so still a fallback rather
                        // than a failure.
                        Err(ShotFailure::NoBrowser(err)) => {
                            format!("the browser would not start ({err})")
                        }
                        // The browser ran and the deck did not render. Not a
                        // reason to publish a different overview and say
                        // nothing.
                        Err(err @ ShotFailure::Rendering(_)) => return Err(err.into()),
                    }
                }
            };

            // Reported by the caller, not here: the server logs it, the CLI
            // prints it, and neither wants the other's channel. See `Drawn`.
            typst_thumbnails(talk, mermaid, out_dir).await?;
            Ok(Drawn::FellBackToTypst(reason))
        }
    }
}

/// The Typst renderer, off the async runtime: it shells out to `typst` once per
/// slide and blocks on each.
async fn typst_thumbnails(
    talk: &Talk,
    mermaid: &MermaidRenderer,
    out_dir: &Path,
) -> anyhow::Result<()> {
    let (talk, mermaid, out_dir) = (talk.clone(), mermaid.clone(), out_dir.to_path_buf());
    tokio::task::spawn_blocking(move || render_typst_thumbnails(&talk, &out_dir, &mermaid))
        .await?
        .map_err(|err| anyhow::anyhow!("{err}"))
}
