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
use tracing::{info, warn};

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
/// # Errors
/// Returns an error if the thumbnails cannot be drawn — no browser and no
/// `typst`, or either of them failing — or the overview page cannot be written.
pub async fn generate_overview(
    talk: &Talk,
    mermaid: &MermaidRenderer,
    out_dir: &Path,
    options: &OverviewOptions,
) -> anyhow::Result<()> {
    render_thumbnails(talk, mermaid, out_dir, options).await?;

    let thumbnails = ThumbnailOptions {
        search: options.search,
        static_links: options.static_links,
        ..ThumbnailOptions::new(mermaid.clone())
    };
    let (talk, out_dir) = (talk.clone(), out_dir.to_path_buf());
    tokio::task::spawn_blocking(move || write_overview_page(&talk, &out_dir, &thumbnails))
        .await?
        .map_err(|err| anyhow::anyhow!("{err}"))
}

async fn render_thumbnails(
    talk: &Talk,
    mermaid: &MermaidRenderer,
    out_dir: &Path,
    options: &OverviewOptions,
) -> anyhow::Result<()> {
    let shots = ShotOptions {
        browser: options.browser.clone(),
        public_dir: options.public_dir.clone(),
    };

    match options.renderer {
        ThumbnailRenderer::Browser => Ok(shoot_slides(talk, mermaid, out_dir, &shots).await?),
        ThumbnailRenderer::Typst => typst_thumbnails(talk, mermaid, out_dir).await,
        ThumbnailRenderer::Auto => {
            let Some(browser) = find_browser(options.browser.as_deref()) else {
                fell_back("no Chrome, Chromium or Edge could be started");
                return typst_thumbnails(talk, mermaid, out_dir).await;
            };
            info!(browser = %browser.display(), "Photographing the deck for the overview");
            match shoot_slides(talk, mermaid, out_dir, &shots).await {
                Ok(()) => Ok(()),
                // Detection said yes and the launch said no — a container
                // without the shared libraries, most likely. Still a missing
                // browser, so still a fallback rather than a failure.
                Err(ShotFailure::NoBrowser(err)) => {
                    fell_back(&format!("the browser would not start ({err})"));
                    typst_thumbnails(talk, mermaid, out_dir).await
                }
                // The browser ran and the deck did not render. Not a reason to
                // publish a different overview and say nothing.
                Err(err @ ShotFailure::Rendering(_)) => Err(err.into()),
            }
        }
    }
}

/// Says, once and loudly, what the deck is about to lose.
fn fell_back(reason: &str) {
    warn!(
        "The slide overview falls back to Typst because {reason}. Typst cannot render a \
         deck's HTML, CSS or terminals, so the thumbnails will not match the projector — \
         install a browser, set CHROME, or pass --browser."
    );
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
