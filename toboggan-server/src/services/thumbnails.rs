//! Lazily-generated slide-overview thumbnails.
//!
//! The everyday `toboggan <folder>` serve does not pre-render thumbnails (that
//! would add multi-second startup latency and a hard dependency on the `typst`
//! binary). Instead the overview is generated on the first `/slides` request,
//! cached in a temp dir, and invalidated whenever the talk reloads. When `typst`
//! is absent or fails, the service records the reason and the page degrades
//! gracefully rather than erroring.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tempfile::TempDir;
use toboggan_cli::output::{ThumbnailOptions, generate_thumbnails};
use tokio::sync::RwLock;
use tracing::{error, info};

use super::TalkService;

/// Lazily-generated slide-overview thumbnails, regenerated after each reload.
#[derive(Clone)]
pub(crate) struct ThumbnailService {
    state: Arc<RwLock<ThumbState>>,
}

enum ThumbState {
    /// Nothing generated yet; the next request kicks off generation.
    Idle,
    /// Generation is in flight.
    Generating,
    /// Thumbnails are available in `Source`.
    Ready(Source),
    /// Generation failed (e.g. `typst` is missing); holds a user-facing reason.
    Unavailable(String),
}

enum Source {
    /// Generated on demand into a temp dir (auto-removed on drop).
    Owned(Arc<TempDir>),
    /// Pre-supplied via `--thumbnails-dir`; kept across reloads.
    External(PathBuf),
}

impl Source {
    fn path(&self) -> &Path {
        match self {
            Self::Owned(dir) => dir.path(),
            Self::External(path) => path,
        }
    }
}

/// What the `/slides` page should render right now.
pub(crate) enum ThumbStatus {
    /// Thumbnails are ready; serve the overview.
    Ready,
    /// Generation is underway; show a "generating…" page that retries.
    Pending,
    /// Generation is not possible; show the reason.
    Unavailable(String),
}

impl ThumbnailService {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(ThumbState::Idle)),
        }
    }

    /// Pre-seeds a ready, externally-generated thumbnails directory.
    pub(crate) async fn seed_external(&self, dir: PathBuf) {
        *self.state.write().await = ThumbState::Ready(Source::External(dir));
    }

    /// Resets to [`ThumbState::Idle`] so the next request regenerates after a
    /// talk reload. An externally-supplied directory is kept as the source.
    pub(crate) async fn invalidate(&self) {
        let mut state = self.state.write().await;
        if matches!(&*state, ThumbState::Ready(Source::External(_))) {
            return;
        }
        *state = ThumbState::Idle;
    }

    /// Ensures generation is underway (using `talk_service`) and reports status.
    pub(crate) async fn ensure(&self, talk_service: TalkService) -> ThumbStatus {
        if let Some(status) = self.snapshot().await {
            return status;
        }
        // Transition Idle -> Generating, re-checking under the write lock to
        // avoid spawning two generators on concurrent first requests.
        {
            let mut state = self.state.write().await;
            match &*state {
                ThumbState::Idle => *state = ThumbState::Generating,
                ThumbState::Generating => return ThumbStatus::Pending,
                ThumbState::Ready(_) => return ThumbStatus::Ready,
                ThumbState::Unavailable(reason) => {
                    return ThumbStatus::Unavailable(reason.clone());
                }
            }
        }
        self.spawn(talk_service);
        ThumbStatus::Pending
    }

    /// Reads a generated asset (e.g. `overview.html`, `thumb-0001.png`) by its
    /// relative path. Returns `None` unless thumbnails are ready, and rejects any
    /// path that tries to escape the cache directory.
    pub(crate) async fn read_asset(&self, rel: &str) -> Option<Vec<u8>> {
        if rel
            .split('/')
            .any(|segment| segment == ".." || segment.is_empty())
        {
            return None;
        }
        let state = self.state.read().await;
        match &*state {
            ThumbState::Ready(source) => std::fs::read(source.path().join(rel)).ok(),
            _ => None,
        }
    }

    /// Returns a status snapshot without transitioning out of `Idle`.
    async fn snapshot(&self) -> Option<ThumbStatus> {
        let state = self.state.read().await;
        match &*state {
            ThumbState::Ready(_) => Some(ThumbStatus::Ready),
            ThumbState::Generating => Some(ThumbStatus::Pending),
            ThumbState::Unavailable(reason) => Some(ThumbStatus::Unavailable(reason.clone())),
            ThumbState::Idle => None,
        }
    }

    fn spawn(&self, talk_service: TalkService) {
        let state = self.state.clone();
        tokio::spawn(async move {
            let talk = talk_service.talk().await;
            let result = tokio::task::spawn_blocking(move || generate(&talk)).await;

            let mut guard = state.write().await;
            // A reload may have reset us to Idle mid-flight; leave it alone so the
            // next request regenerates against the new talk.
            if !matches!(&*guard, ThumbState::Generating) {
                return;
            }
            *guard = match result {
                Ok(Ok(dir)) => {
                    info!("generated slide-overview thumbnails");
                    ThumbState::Ready(Source::Owned(Arc::new(dir)))
                }
                Ok(Err(err)) => {
                    error!("thumbnail generation failed: {err:?}");
                    ThumbState::Unavailable(unavailable_message(&err))
                }
                Err(join) => {
                    error!("thumbnail generation task crashed: {join}");
                    ThumbState::Unavailable("thumbnail generation crashed".to_owned())
                }
            };
        });
    }
}

fn generate(talk: &toboggan_core::Talk) -> anyhow::Result<TempDir> {
    let dir = tempfile::tempdir()?;
    generate_thumbnails(talk, dir.path(), ThumbnailOptions::default())
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    Ok(dir)
}

fn unavailable_message(err: &anyhow::Error) -> String {
    format!("Slide thumbnails could not be generated (is the `typst` binary installed?): {err}")
}
