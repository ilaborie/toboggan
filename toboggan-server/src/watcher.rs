use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::TobogganState;

const DEBOUNCE_DURATION: Duration = Duration::from_millis(300);

pub fn start_watch_task(talk_path: PathBuf, state: TobogganState) -> anyhow::Result<()> {
    info!(path = %talk_path.display(), "Starting file watcher");

    let (tx, mut rx) = mpsc::channel::<Result<Event, notify::Error>>(100);

    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        if tx.blocking_send(res).is_err() {
            error!("Failed to send file watcher event - channel closed");
        }
    })
    .context("Failed to create file watcher")?;

    watcher
        .watch(&talk_path, RecursiveMode::NonRecursive)
        .with_context(|| format!("Failed to watch file: {}", talk_path.display()))?;

    tokio::spawn(async move {
        let mut last_reload = tokio::time::Instant::now();
        let _watcher = watcher; // Keep watcher alive

        while let Some(event_result) = rx.recv().await {
            match event_result {
                Ok(event) => {
                    if should_reload(&event) {
                        let now = tokio::time::Instant::now();
                        let elapsed = now.duration_since(last_reload);

                        if elapsed >= DEBOUNCE_DURATION {
                            info!("File change detected, reloading talk");
                            if let Err(err) = reload_talk(&talk_path, &state).await {
                                error!("Failed to reload talk: {err:?}");
                            } else {
                                last_reload = now;
                            }
                        }
                    }
                }
                Err(err) => {
                    warn!("File watcher error: {err}");
                }
            }
        }

        info!("File watcher task stopped");
    });

    Ok(())
}

fn should_reload(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
    )
}

async fn reload_talk(path: &Path, state: &TobogganState) -> anyhow::Result<()> {
    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Reading talk file {}", path.display()))?;

    let new_talk = toml::from_str(&content).context("Parsing talk TOML")?;

    state.reload_talk(new_talk).await?;

    Ok(())
}
