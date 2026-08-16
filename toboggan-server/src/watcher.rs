use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};
use toboggan_core::Talk;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::TobogganState;

/// How long the watched paths must be quiet before a reload runs.
const DEBOUNCE_DURATION: Duration = Duration::from_millis(300);

/// Rebuilds a fresh [`Talk`] from the current on-disk content of a watched path.
///
/// Used to decouple the watcher from *how* a talk is produced: serving a single
/// `.toml` file reads and parses that file, while a folder-based presentation
/// re-runs the folder parser.
pub type ReloadFn = Box<dyn Fn() -> anyhow::Result<Talk> + Send + Sync + 'static>;

/// What the watcher observes.
///
/// An enum rather than `paths: Vec<PathBuf>` + `recursive: bool`, which could
/// express combinations that do not exist (a recursive single file, a
/// non-recursive deck) and left the rule in a doc comment instead of the type.
pub enum WatchTarget {
    /// A single prebuilt `.toml` talk file.
    TalkFile(PathBuf),
    /// A deck: its slides folder, plus the `public/` assets directory when the
    /// deck has one.
    ///
    /// An asset edit does not change the [`Talk`], but running the same reload
    /// path still notifies clients, and a client that re-renders re-fetches the
    /// stylesheets and images it references — which is why `/public` must be
    /// served with revalidation for this to be visible.
    Deck {
        /// The folder the parser walks.
        slides: PathBuf,
        /// The deck's `public/` directory, when it exists.
        assets: Option<PathBuf>,
    },
}

impl WatchTarget {
    /// The paths to hand the file-system watcher.
    fn paths(&self) -> Vec<&Path> {
        match self {
            Self::TalkFile(path) => vec![path.as_path()],
            Self::Deck { slides, assets } => [Some(slides.as_path()), assets.as_deref()]
                .into_iter()
                .flatten()
                .collect(),
        }
    }

    /// Whether the paths are watched recursively. Only a deck is a tree.
    fn recursive(&self) -> RecursiveMode {
        match self {
            Self::TalkFile(_) => RecursiveMode::NonRecursive,
            Self::Deck { .. } => RecursiveMode::Recursive,
        }
    }
}

/// Configuration for the talk reload watcher.
pub struct WatchConfig {
    /// What to watch.
    pub target: WatchTarget,
    /// Rebuilds the [`Talk`] from the current on-disk content.
    pub reload: ReloadFn,
}

/// Starts a background task that watches `config.paths` and hot-swaps the served
/// talk once a burst of changes has settled (see [`watch_loop`]).
///
/// # Errors
/// Returns an error if the underlying file-system watcher cannot be created or
/// cannot start watching one of the requested paths.
pub fn start_watch_task(config: WatchConfig, state: TobogganState) -> anyhow::Result<()> {
    let WatchConfig { target, reload } = config;
    let paths = target.paths();
    let mode = target.recursive();
    info!(?paths, ?mode, "Starting talk watcher");

    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>(100);

    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        if tx.blocking_send(res).is_err() {
            error!("Failed to send file watcher event - channel closed");
        }
    })
    .context("Failed to create file watcher")?;

    for path in &paths {
        watcher
            .watch(path, mode)
            .with_context(|| format!("Failed to watch path: {}", path.display()))?;
    }

    tokio::spawn(watch_loop(watcher, rx, state, reload));

    Ok(())
}

/// Collects change events and reloads once the burst has settled.
///
/// Trailing edge, not leading: an atomic-save editor writes a temp file and then
/// renames it over the original, and a leading-edge throttle reloaded on the
/// *first* of those events — re-parsing the slide's pre-save content — then
/// discarded the rename with no retry. The presenter saved a slide and the
/// browser kept showing the old text until some unrelated file was touched.
/// Waiting for `DEBOUNCE_DURATION` of quiet coalesces the burst into one reload
/// of the final content.
async fn watch_loop(
    watcher: RecommendedWatcher,
    mut rx: mpsc::Receiver<Result<Event, notify::Error>>,
    state: TobogganState,
    reload: ReloadFn,
) {
    let _watcher = watcher; // Keep watcher alive
    let mut settle_at: Option<tokio::time::Instant> = None;

    loop {
        let event = match settle_at {
            Some(deadline) => tokio::select! {
                event = rx.recv() => event,
                () = tokio::time::sleep_until(deadline) => {
                    settle_at = None;
                    do_reload(&state, &reload).await;
                    continue;
                }
            },
            None => rx.recv().await,
        };

        match event {
            Some(Ok(event)) => {
                if should_reload(&event) {
                    // Push the deadline out: the burst is still arriving.
                    settle_at = Some(tokio::time::Instant::now() + DEBOUNCE_DURATION);
                }
            }
            Some(Err(error)) => warn!(?error, "File watcher error"),
            None => break,
        }
    }

    info!("File watcher task stopped");
}

async fn do_reload(state: &TobogganState, reload: &ReloadFn) {
    info!("Change detected, reloading talk");
    let new_talk = match reload() {
        Ok(talk) => talk,
        Err(err) => {
            error!("Failed to rebuild talk: {err:?}");
            return;
        }
    };

    if let Err(err) = state.reload_talk(new_talk).await {
        error!("Failed to reload talk: {err:?}");
    }
}

fn should_reload(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
    )
}
