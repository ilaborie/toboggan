//! Producing a slide-overview directory: the thumbnails, and the page around
//! them.
//!
//! One entry point for the two callers that want one — the lazily-generated
//! overview behind `/slides`, and `toboggan thumbnails` — so the choice of
//! renderer is made in one place and behaves the same from both.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
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

/// The renderer choice once it has been made, with the browser it will use.
///
/// [`ThumbnailRenderer`] is what the *caller asked for*; this is what will
/// actually happen, and it is reached by running browser detection exactly once.
/// The question used to be asked three times over — by the CLI's pre-flight
/// check, by `render_thumbnails`, and again inside `shots::launch` — each
/// spawning a `--version` per candidate, and each free to come back with a
/// different answer than the last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedRenderer {
    /// Photograph the deck, with this browser.
    Photograph(PathBuf),
    /// Redraw it with Typst, which is what was asked for.
    Typst,
    /// Redraw it with Typst because [`ThumbnailRenderer::Auto`] found no browser.
    /// Carries the reason, for [`Drawn::FellBackToTypst`].
    TypstInstead(String),
}

impl ThumbnailRenderer {
    /// Decides how the deck gets drawn, looking for a browser at most once.
    ///
    /// Detection shells out once per candidate, so it runs on a blocking thread
    /// rather than on the runtime the server is answering requests with.
    ///
    /// # Errors
    /// [`ThumbnailRenderer::Browser`] when no browser can be found — the whole
    /// point of asking for it by name rather than taking `auto`.
    pub async fn resolve(self, browser: Option<&Path>) -> anyhow::Result<ResolvedRenderer> {
        if matches!(self, Self::Typst) {
            return Ok(ResolvedRenderer::Typst);
        }

        let pinned = browser.map(Path::to_path_buf);
        let found = tokio::task::spawn_blocking(move || find_browser(pinned.as_deref()))
            .await
            .context("Looking for a browser to photograph the deck with")?;

        match found {
            Some(browser) => Ok(ResolvedRenderer::Photograph(browser)),
            None if matches!(self, Self::Browser) => bail!(
                "no Chrome, Chromium or Edge could be started for rendering slide \
                 thumbnails. Install one, set CHROME, or pass --browser"
            ),
            None => Ok(ResolvedRenderer::TypstInstead(
                "no Chrome, Chromium or Edge could be started".to_owned(),
            )),
        }
    }
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
    let browser = match options.renderer.resolve(options.browser.as_deref()).await? {
        ResolvedRenderer::Typst => {
            typst_thumbnails(talk, mermaid, out_dir).await?;
            return Ok(Drawn::Redrawn);
        }
        ResolvedRenderer::TypstInstead(reason) => {
            typst_thumbnails(talk, mermaid, out_dir).await?;
            // Reported by the caller, not here: the server logs it, the CLI
            // prints it, and neither wants the other's channel. See `Drawn`.
            return Ok(Drawn::FellBackToTypst(reason));
        }
        ResolvedRenderer::Photograph(browser) => browser,
    };

    info!(browser = %browser.display(), "Photographing the deck for the overview");
    let shots = ShotOptions {
        browser,
        public_dir: options.public_dir.clone(),
    };

    match shoot_slides(talk, mermaid, out_dir, &shots).await {
        Ok(()) => Ok(Drawn::Photographed),
        // Detection said yes and the launch said no — a container without the
        // shared libraries, most likely. Still a missing browser, so still a
        // fallback rather than a failure, but only where a fallback was on
        // offer: `--thumbnail-renderer browser` asked for a photograph by name.
        Err(ShotFailure::NoBrowser(err)) if options.renderer == ThumbnailRenderer::Auto => {
            let reason = format!("the browser would not start ({err})");
            // Both causes, or the report names Typst for a run that failed
            // because of the browser. The pre-flight check cannot have covered
            // this path: it only looks for `typst` when the *decision* was Typst,
            // and here the decision was a photograph right up until the launch.
            typst_thumbnails(talk, mermaid, out_dir)
                .await
                .with_context(|| format!("Falling back to Typst because {reason}"))?;
            Ok(Drawn::FellBackToTypst(reason))
        }
        // The browser ran and the deck did not render. Not a reason to publish a
        // different overview and say nothing.
        Err(err) => Err(err.into()),
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// `/bin/echo` answers `--version` with a zero exit, which is all
    /// `find_browser` asks of a candidate — so it stands in for a browser
    /// without one being installed.
    const A_WORKING_BROWSER: &str = "/bin/echo";
    const NO_SUCH_BROWSER: &str = "/nope/not-a-browser";

    /// Asking for Typst is not a question about browsers, so it must not go
    /// looking for one — this is the arm that keeps `--thumbnail-renderer typst`
    /// free of a filesystem sweep it has no use for.
    #[tokio::test]
    async fn typst_is_decided_without_looking_for_a_browser() {
        let resolved = ThumbnailRenderer::Typst
            .resolve(Some(Path::new(NO_SUCH_BROWSER)))
            .await
            .expect("typst needs no browser");

        assert_eq!(resolved, ResolvedRenderer::Typst);
    }

    /// `browser` means "photograph it, and tell me if you cannot". Resolving up
    /// front is what moves that failure ahead of parsing the deck.
    #[tokio::test]
    async fn an_absent_browser_fails_the_strict_renderer() {
        let err = ThumbnailRenderer::Browser
            .resolve(Some(Path::new(NO_SUCH_BROWSER)))
            .await
            .expect_err("browser must not fall back");

        assert!(
            err.to_string().contains("no Chrome"),
            "the error should name what is missing: {err}"
        );
    }

    /// …while `auto` takes the same absence as its cue to redraw, and carries
    /// the reason out so the caller can say what the deck lost.
    #[tokio::test]
    async fn an_absent_browser_sends_auto_to_typst_with_a_reason() {
        let resolved = ThumbnailRenderer::Auto
            .resolve(Some(Path::new(NO_SUCH_BROWSER)))
            .await
            .expect("auto falls back rather than failing");

        match resolved {
            ResolvedRenderer::TypstInstead(reason) => assert!(
                reason.contains("no Chrome"),
                "the reason reaches the user verbatim: {reason}"
            ),
            other => panic!("expected a fallback, got {other:?}"),
        }
    }

    /// Both photographing modes come back with the path itself, which is what
    /// lets `ShotOptions` carry it to the launch instead of asking again.
    #[tokio::test]
    async fn a_working_browser_is_carried_out_by_path() {
        for renderer in [ThumbnailRenderer::Auto, ThumbnailRenderer::Browser] {
            let resolved = renderer
                .resolve(Some(Path::new(A_WORKING_BROWSER)))
                .await
                .expect("a usable browser resolves");

            assert_eq!(
                resolved,
                ResolvedRenderer::Photograph(PathBuf::from(A_WORKING_BROWSER)),
                "{renderer:?} should photograph with the browser it was given"
            );
        }
    }
}
