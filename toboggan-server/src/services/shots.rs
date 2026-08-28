//! Per-slide PNGs, photographed from the deck itself.
//!
//! The overview at `/slides` used to be illustrated by a *second* rendering of
//! the deck: the same markdown put through a Typst document instead of a
//! browser. It could only ever approximate. Typst has no `<style>`, no raw HTML
//! and no terminals, so `output/typst.rs` drops all three on the floor — a deck
//! that leans on any of them (and a deck fixing its own layout slide by slide
//! leans on the first) had thumbnails that did not match the projector, without
//! ever failing loudly about it.
//!
//! So the picture is taken rather than redrawn: a headless Chrome opens
//! `/run?shot=N` against a private copy of the server and photographs it. What
//! the overview shows and what the room sees are then the same rendering by
//! construction, and the entire parallel renderer stops being load-bearing.
//!
//! Three things this deliberately does not do:
//!
//! * **Download a browser.** `chromiumoxide`'s fetcher feature is off. A browser
//!   already on the machine is used; otherwise the caller falls back to Typst.
//!   A presentation tool must not quietly pull 150 MB of Chromium down a
//!   conference wifi.
//! * **Shoot the live server.** See [`crate::serve_ephemeral`] for why.
//! * **Photograph a slide before it has settled.** The page says when it is
//!   ready ([`SHOT_READY_ATTRIBUTE`]); a slide that never does is reported, not
//!   filed as a blank rectangle.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context as _, bail};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::handler::viewport::Viewport;
use chromiumoxide::page::{Page, ScreenshotParams};
use futures::StreamExt as _;
use toboggan_cli::mermaid::MermaidRenderer;
use toboggan_core::Talk;
use tracing::{debug, info};

/// The attribute the shot page puts on `<html>` when it has finished painting.
///
/// Its two values are the contract with `toboggan-wasm`'s `components/shot`:
/// `ready` when fonts have settled and images have decoded, `error` when the
/// slide could not be fetched at all. Anything else means the page is still
/// working.
const SHOT_READY_ATTRIBUTE: &str = "data-toboggan-shot";

/// How long one slide gets to report itself ready.
///
/// Generous, because the first slide of a run pays for the wasm bundle as well
/// as itself, and a cold Chrome on a laptop that is also running the talk is not
/// a fast machine. Every slide after the first is served from the browser's own
/// cache.
const SHOT_TIMEOUT: Duration = Duration::from_secs(30);

/// How often the driver asks the page whether it is ready.
const SHOT_POLL: Duration = Duration::from_millis(50);

/// The projector, in CSS pixels.
///
/// The same 1280x720 the presenter view lays its mirrors out at, and for the
/// same reason: a deck breaks its lines against the viewport it is given, so a
/// preview at another size is a preview of a different slide.
const SHOT_WIDTH: u32 = 1280;
const SHOT_HEIGHT: u32 = 720;

/// How the projector's pixels map to the PNG's.
///
/// Half: the overview grid draws its cards at 240 CSS pixels wide, so a
/// 1280-pixel-wide capture is four times more image than any of it is ever
/// shown at. Laid out at [`SHOT_WIDTH`] and captured at 640x360, the thumbnail
/// stays sharp on a retina display and the overview directory stays a quarter of
/// the size.
const SHOT_SCALE: f64 = 0.5;

/// Where the browser comes from, and what it is pointed at.
#[derive(Debug, Clone, Default)]
pub struct ShotOptions {
    /// An explicit browser binary, overriding detection.
    pub browser: Option<PathBuf>,
    /// The deck's `public/` directory, so a slide's `<img src="/public/…">`
    /// resolves. Without it every deck picture is photographed as a broken
    /// image icon.
    pub public_dir: Option<PathBuf>,
}

/// Finds the browser that would be used, if there is one.
///
/// The question `--thumbnail-renderer auto` answers before deciding whether to
/// shoot the deck or fall back to Typst. `browser` short-circuits detection —
/// and is not replaced by something else when it is wrong, because
/// `--browser /opt/chrome-131` quietly shooting with Chrome 145 is worse than
/// saying so.
///
/// Every candidate is asked for its `--version` before being believed. Existing
/// on disk is not enough: a stale Homebrew cask leaves an executable
/// `chromium.wrapper.sh` on the `PATH` pointing at an application that has been
/// deleted, and *that* is what plain detection returns — so the overview failed
/// on a machine with a perfectly good Chrome in `/Applications`.
#[must_use]
pub fn find_browser(browser: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = browser {
        return usable(path).then(|| path.to_path_buf());
    }
    candidates().into_iter().find(|path| usable(path))
}

/// Every browser worth trying, best first.
fn candidates() -> Vec<PathBuf> {
    // `CHROME` is how a caller says "this one", and the same variable Puppeteer
    // and its descendants read. Read here and passed down so the ordering below
    // can be tested without a test mutating the process environment — which this
    // workspace could not do anyway, since it denies `unsafe`.
    candidates_from(std::env::var_os("CHROME").map(PathBuf::from))
}

fn candidates_from(pinned: Option<PathBuf>) -> Vec<PathBuf> {
    let mut paths = pinned.into_iter().collect::<Vec<_>>();

    // Bare names, resolved by the OS against `PATH` when they are run. Chrome
    // before Chromium before the others, so a machine with several installed
    // photographs the deck in the one most of the room is using.
    paths.extend(
        [
            "google-chrome-stable",
            "google-chrome",
            "chrome",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
            "brave-browser",
        ]
        .into_iter()
        .map(PathBuf::from),
    );

    // The usual installs, which are not on anyone's `PATH` on macOS.
    paths.extend(
        [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
            "/usr/bin/google-chrome",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/usr/bin/microsoft-edge",
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ]
        .into_iter()
        .map(PathBuf::from),
    );

    paths
}

/// Whether `path` is a browser that actually runs.
fn usable(path: &Path) -> bool {
    std::process::Command::new(path)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Photographs every slide of `talk` into `out_dir` as `thumb-NNNN.png`.
///
/// One file per slide over the FULL slide list, no render-target filtering, so
/// page N is slide N — the overview's click-to-run targets are indexes into
/// `/api/slides` and must stay aligned with it.
///
/// # Errors
/// Returns [`ShotFailure::NoBrowser`] if none can be started, and
/// [`ShotFailure::Rendering`] if one could but the deck could not be
/// photographed. The two are separate because only the first is a reason to
/// quietly fall back to Typst — a browser that *did* run and then failed is a
/// bug, and swapping in the approximate renderer would hide it.
pub async fn shoot_slides(
    talk: &Talk,
    mermaid: &MermaidRenderer,
    out_dir: &Path,
    options: &ShotOptions,
) -> Result<(), ShotFailure> {
    tokio::fs::create_dir_all(out_dir)
        .await
        .with_context(|| format!("Creating {}", out_dir.display()))
        .map_err(ShotFailure::Rendering)?;

    let server =
        crate::serve_ephemeral(shootable(talk), mermaid.clone(), options.public_dir.clone())
            .await
            .context("Starting the private server the shots are taken against")
            .map_err(ShotFailure::Rendering)?;
    let origin = server.origin();

    // The server is stopped on every path out, including the error ones, which
    // is why the work is a separate future rather than a string of `?`s here.
    let result = shoot_all(talk.slides.len(), &origin, out_dir, options).await;
    server.stop().await;
    result
}

/// The deck as the overview needs it: every slide, hiding nothing.
///
/// `TalkService` drops `hidden_in = ["web"]` slides — a slide the room does not
/// see is not a slide the room can be told to go to — and renumbers what is
/// left. The overview is the other kind of view: an authoring one, which lists
/// every slide and badges the ones the web will not show, and whose whole
/// contract is that `thumb-NNNN.png` is slide N of the deck *as authored*.
///
/// Handing the private server the authored deck was not enough, because it
/// applies that filter again on the way in. So the markers are cleared first,
/// which makes the filter a no-op and leaves `/api/slides/{index}` indexing the
/// list the thumbnails are numbered over.
///
/// Without this the guide — one `hidden_in = ["web"]` slide among 44 — asked for
/// slide 43 of a 43-slide list and failed; and every slide *after* the hidden
/// one would have been photographed one place out, filed under the right name.
/// The loud failure was the lucky half.
fn shootable(talk: &Talk) -> Talk {
    let mut talk = talk.clone();
    for slide in &mut talk.slides {
        slide.hidden_in.clear();
    }
    talk
}

/// Why a deck could not be photographed.
#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum ShotFailure {
    /// No browser could be found or started.
    #[display("{_0}")]
    NoBrowser(#[error(source)] anyhow::Error),
    /// A browser started, but the deck could not be photographed.
    #[display("{_0}")]
    Rendering(#[error(source)] anyhow::Error),
}

async fn shoot_all(
    slides: usize,
    origin: &str,
    out_dir: &Path,
    options: &ShotOptions,
) -> Result<(), ShotFailure> {
    // A profile of this run's own, kept alive until the browser has exited.
    //
    // The library's default is one fixed path — `$TMPDIR/chromiumoxide-runner`
    // — shared by every process on the machine, and Chrome refuses to start a
    // second instance against a profile another instance holds ("Aborting now
    // to avoid profile corruption"). Two decks open at once, or a `toboggan
    // thumbnails` beside a running server, is not an exotic thing to do.
    let profile = tempfile::tempdir()
        .context("Creating a browser profile directory")
        .map_err(ShotFailure::Rendering)?;
    let (browser, mut handler) = launch(options, profile.path())
        .await
        .map_err(ShotFailure::NoBrowser)?;

    // chromiumoxide's connection is driven by whoever polls the handler; nothing
    // else in this function would. Aborted below rather than left to run, since
    // it outlives the browser it is talking to.
    let pump = tokio::spawn(async move { while handler.next().await.is_some() {} });

    let result = shoot_pages(&browser, slides, origin, out_dir)
        .await
        .map_err(ShotFailure::Rendering);

    // Best-effort: the browser is a child process we started, and a failure to
    // close it politely is not a reason to lose thumbnails we did take.
    let mut browser = browser;
    if let Err(err) = browser.close().await {
        debug!(%err, "Could not close the browser cleanly");
    }
    let _ = browser.wait().await;
    pump.abort();

    result
}

async fn shoot_pages(
    browser: &Browser,
    slides: usize,
    origin: &str,
    out_dir: &Path,
) -> anyhow::Result<()> {
    // One tab for the whole deck. A tab per slide would re-instantiate the wasm
    // client every time, which is most of the cost of a shot.
    let page = browser
        .new_page("about:blank")
        .await
        .context("Opening the page the slides are shot in")?;

    for index in 0..slides {
        let url = format!("{origin}/run?shot={index}");
        page.goto(&url)
            .await
            .with_context(|| format!("Opening {url}"))?;
        await_ready(&page, index).await?;

        let png = page
            .screenshot(
                ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    // The slide fills the viewport, and anything below it is the
                    // page's own background rather than more deck.
                    .full_page(false)
                    .build(),
            )
            .await
            .with_context(|| format!("Photographing slide {index}"))?;

        let path = out_dir.join(format!("thumb-{index:04}.png"));
        tokio::fs::write(&path, png)
            .await
            .with_context(|| format!("Writing {}", path.display()))?;
    }

    let _ = page.close().await;
    info!(slides, "Photographed the deck for the slide overview");
    Ok(())
}

/// Waits for the shot page to say it has finished painting.
///
/// A timeout here is a real failure and is reported as one. The alternative —
/// shoot anyway after N seconds — is how an overview fills up with blank cards
/// that nobody can tell from slides that are genuinely blank.
async fn await_ready(page: &Page, index: usize) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + SHOT_TIMEOUT;
    let expression =
        format!("document.documentElement.getAttribute('{SHOT_READY_ATTRIBUTE}') ?? ''");

    loop {
        let state = page
            .evaluate_expression(expression.as_str())
            .await
            .ok()
            .and_then(|result| result.into_value::<String>().ok())
            .unwrap_or_default();

        match state.as_str() {
            "ready" => return Ok(()),
            "error" => {
                bail!("slide {index} could not be rendered; the browser console has the reason")
            }
            _ => {}
        }

        if tokio::time::Instant::now() >= deadline {
            bail!(
                "slide {index} never finished rendering (waited {}s)",
                SHOT_TIMEOUT.as_secs()
            );
        }
        tokio::time::sleep(SHOT_POLL).await;
    }
}

async fn launch(
    options: &ShotOptions,
    profile: &Path,
) -> anyhow::Result<(Browser, chromiumoxide::handler::Handler)> {
    let config = BrowserConfig::builder()
        .user_data_dir(profile)
        // The *new* headless mode: the real browser, rather than the separate
        // and divergent renderer the old one used — which matters rather a lot
        // when the whole point is that the thumbnail matches what a person sees.
        .new_headless_mode()
        // Laid out at projector size, captured at half of it. See SHOT_SCALE.
        .viewport(Viewport {
            width: SHOT_WIDTH,
            height: SHOT_HEIGHT,
            device_scale_factor: Some(SHOT_SCALE),
            emulating_mobile: false,
            is_landscape: true,
            has_touch: false,
        })
        .window_size(SHOT_WIDTH, SHOT_HEIGHT)
        // A scrollbar is not part of the slide, and a slide that overflows by a
        // pixel would otherwise be photographed with one down its side.
        .arg("--hide-scrollbars")
        // Same colours on every machine, so a thumbnail regenerated elsewhere is
        // the same image.
        .arg("--force-color-profile=srgb")
        // The page is one we wrote, served over loopback, on a port only this
        // process knows. The sandbox buys nothing here and is unavailable in the
        // containers CI runs in.
        .no_sandbox();

    // Resolved here rather than left to the library's own detection, which
    // believes the first candidate that exists on disk. See `find_browser`.
    let executable = find_browser(options.browser.as_deref()).ok_or_else(|| {
        anyhow::anyhow!(
            "no Chrome, Chromium or Edge could be started for rendering slide thumbnails. \
             Install one, set CHROME, or pass --browser"
        )
    })?;
    let config = config
        .chrome_executable(&executable)
        .build()
        .map_err(|err| anyhow::anyhow!("{}: {err}", executable.display()))?;

    Browser::launch(config)
        .await
        .context("Launching the browser that photographs the deck")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// An explicit path that is not there is a mistake worth surfacing as
    /// "no browser", not silently replaced by whatever else is installed —
    /// otherwise `--browser /opt/chrome-131` quietly shoots with Chrome 145.
    #[test]
    fn an_explicit_browser_that_does_not_exist_is_not_replaced() {
        assert_eq!(find_browser(Some(Path::new("/nope/chrome"))), None);
    }

    /// The bug that made this function ours rather than the library's: the
    /// candidate existed, was executable, and still could not run — a Homebrew
    /// cask wrapper left behind by a deleted application.
    #[test]
    fn a_candidate_that_cannot_run_is_not_a_browser() {
        assert!(!usable(Path::new("/definitely/not/a/browser")));
        // Executable, exits non-zero for `--version`.
        assert!(!usable(Path::new("/usr/bin/false")));
    }

    /// Something that does answer `--version` is believed, whatever it is: the
    /// check is "does this run", not "is this really Chrome".
    #[test]
    fn a_candidate_that_answers_version_is_used_verbatim() {
        let echo = Path::new("/bin/echo");
        assert!(usable(echo));
        assert_eq!(find_browser(Some(echo)), Some(echo.to_path_buf()));
    }

    /// `CHROME` is how a caller says "this one", so it is asked first.
    #[test]
    fn the_chrome_variable_leads_the_candidates() {
        let pinned = PathBuf::from("/some/pinned/chrome");
        let candidates = candidates_from(Some(pinned.clone()));
        assert_eq!(candidates.first(), Some(&pinned));
        assert!(candidates.len() > 1, "the rest are still tried after it");
    }

    /// Chrome before Chromium before the rest: a machine with several installed
    /// should photograph the deck in the one most of the room is using.
    #[test]
    fn chrome_outranks_the_other_browsers() {
        let names = candidates_from(None)
            .iter()
            .filter_map(|path| path.to_str().map(str::to_owned))
            .collect::<Vec<_>>();
        let position = |needle: &str| {
            names
                .iter()
                .position(|name| name.contains(needle))
                .unwrap_or(usize::MAX)
        };
        assert!(position("google-chrome") < position("chromium"));
        assert!(position("chromium") < position("edge"));
        assert!(position("chromium") < position("brave"));
    }
}
