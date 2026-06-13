use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};
use toboggan_core::Talk;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::TobogganState;

const DEBOUNCE_DURATION: Duration = Duration::from_millis(300);

/// Rebuilds a fresh [`Talk`] from the current on-disk content of a watched path.
///
/// Used to decouple the watcher from *how* a talk is produced: serving a single
/// `.toml` file reads and parses that file, while a folder-based presentation
/// re-runs the folder parser.
pub type ReloadFn = Box<dyn Fn() -> anyhow::Result<Talk> + Send + Sync + 'static>;

/// Configuration for the talk reload watcher.
pub struct WatchConfig {
    /// Path to watch: a single `.toml` file, or a presentation folder.
    pub path: PathBuf,
    /// Watch the path recursively (used for folder-based presentations).
    pub recursive: bool,
    /// Rebuilds the [`Talk`] from the current on-disk content.
    pub reload: ReloadFn,
}

/// Starts a background task that watches `config.path` and hot-swaps the served
/// talk whenever it changes (debounced).
///
/// # Errors
/// Returns an error if the underlying file-system watcher cannot be created or
/// cannot start watching the requested path.
pub fn start_watch_task(config: WatchConfig, state: TobogganState) -> anyhow::Result<()> {
    let WatchConfig {
        path,
        recursive,
        reload,
    } = config;
    info!(path = %path.display(), recursive, "Starting talk watcher");

    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>(100);

    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        if tx.blocking_send(res).is_err() {
            error!("Failed to send file watcher event - channel closed");
        }
    })
    .context("Failed to create file watcher")?;

    let mode = if recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    watcher
        .watch(&path, mode)
        .with_context(|| format!("Failed to watch path: {}", path.display()))?;

    tokio::spawn(watch_loop(watcher, rx, state, reload));

    Ok(())
}

async fn watch_loop(
    watcher: RecommendedWatcher,
    mut rx: mpsc::Receiver<Result<Event, notify::Error>>,
    state: TobogganState,
    reload: ReloadFn,
) {
    let mut last_reload = tokio::time::Instant::now();
    let _watcher = watcher; // Keep watcher alive

    while let Some(event_result) = rx.recv().await {
        match event_result {
            Ok(event) => {
                if should_reload(&event)
                    && let Some(reload_time) = handle_reload(&state, &reload, last_reload).await
                {
                    last_reload = reload_time;
                }
            }
            Err(error) => {
                warn!(?error, "File watcher error");
            }
        }
    }

    info!("File watcher task stopped");
}

async fn handle_reload(
    state: &TobogganState,
    reload: &ReloadFn,
    last_reload: tokio::time::Instant,
) -> Option<tokio::time::Instant> {
    let now = tokio::time::Instant::now();
    if now.duration_since(last_reload) < DEBOUNCE_DURATION {
        return None;
    }

    info!("Change detected, reloading talk");
    let new_talk = match reload() {
        Ok(talk) => talk,
        Err(err) => {
            error!("Failed to rebuild talk: {err:?}");
            return None;
        }
    };

    match state.reload_talk(new_talk).await {
        Ok(()) => Some(now),
        Err(err) => {
            error!("Failed to reload talk: {err:?}");
            None
        }
    }
}

fn should_reload(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
    )
}
