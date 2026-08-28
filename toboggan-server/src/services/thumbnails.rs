//! Lazily-generated slide-overview thumbnails.
//!
//! The everyday `toboggan <folder>` serve does not pre-render thumbnails (that
//! would add multi-second startup latency, and a hard dependency on a browser or
//! the `typst` binary). Instead the overview is generated on the first `/slides`
//! request, cached in a temp dir, and invalidated whenever the talk reloads.
//! When neither renderer is available, the service records the reason and the
//! page degrades gracefully rather than erroring.
//!
//! *How* the pictures are made is [`super::generate_overview`]'s business; this
//! module owns only when they are made, and what happens to a generation a
//! reload overtakes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tempfile::TempDir;
use toboggan_cli::mermaid::MermaidRenderer;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use super::{OverviewOptions, TalkService, generate_overview};

/// Lazily-generated slide-overview thumbnails, regenerated after each reload.
#[derive(Clone)]
pub(crate) struct ThumbnailService {
    inner: Arc<RwLock<Inner>>,
}

/// The service state plus an `epoch` that is bumped on every [`invalidate`]
/// (a reload). A generation captures the epoch at its start and only commits its
/// result if the epoch is still current, so a reload mid-generation can never
/// publish stale thumbnails.
///
/// The guard is one-directional: a reload landing between the epoch capture and
/// the talk read makes a *fresh* generation look stale and discards it too. That
/// is the safe way round — the next request regenerates — and matches the PDF
/// cache's guard in `crate::state`.
///
/// [`invalidate`]: ThumbnailService::invalidate
struct Inner {
    state: ThumbState,
    epoch: u64,
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

/// The entry page every `/slides` redirect targets; an overview directory
/// without it is unusable.
const OVERVIEW_ENTRY: &str = "overview.html";

/// The outcome of an `/overview/*` asset lookup.
pub(crate) enum AssetLookup {
    /// The asset was read from the overview directory.
    Found(Vec<u8>),
    /// The asset cannot be served: either the thumbnails are ready and it is not
    /// among them, or its path was rejected for trying to escape the cache
    /// directory. Both answer `404`.
    Missing,
    /// Thumbnails are not ready — regenerating after a reload, or unavailable.
    NotReady,
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
            inner: Arc::new(RwLock::new(Inner {
                state: ThumbState::Idle,
                epoch: 0,
            })),
        }
    }

    /// Pre-seeds a ready, externally-generated thumbnails directory.
    ///
    /// Intended for bootstrap (before any request); it overwrites the state
    /// unconditionally and does not coordinate with an in-flight generation.
    ///
    /// A directory without an [`OVERVIEW_ENTRY`] is not seeded: marking it
    /// `Ready` would send `/slides` to a page that does not exist, and the
    /// service falls back to generating the overview itself.
    pub(crate) async fn seed_external(&self, dir: PathBuf) {
        if !dir.join(OVERVIEW_ENTRY).is_file() {
            warn!(
                dir = %dir.display(),
                "thumbnails directory has no {OVERVIEW_ENTRY}; generating the overview instead"
            );
            return;
        }
        self.inner.write().await.state = ThumbState::Ready(Source::External(dir));
    }

    /// Resets to [`ThumbState::Idle`] so the next request regenerates after a
    /// talk reload, and bumps the epoch so any in-flight generation discards its
    /// (now stale) result.
    ///
    /// An externally-supplied directory (`--thumbnails-dir`) is kept as the
    /// source — the operator owns its contents and we must not replace them with
    /// a generated set — but the epoch is still bumped so an in-flight generation
    /// cannot commit over it.
    ///
    /// A previous [`ThumbState::Unavailable`] is cleared: the reason is usually
    /// "`typst` is missing", and pinning that for the process lifetime meant
    /// installing `typst` and reloading never recovered.
    /// A generation already in flight is left in [`ThumbState::Generating`]
    /// rather than reset to `Idle`. The epoch bump above already guarantees it
    /// cannot commit, and its completion handler puts the state back to `Idle`
    /// so the next request regenerates against the current talk — whereas
    /// resetting here published a free slot while the old task was still
    /// running, and the "generating…" page's own two-second refresh took it
    /// immediately. Two generators then ran over the same deck; sharing one
    /// scratch file, they fed `typst` a torn document (`unclosed raw text`) or
    /// deleted it out from under each other (`input file not found`).
    pub(crate) async fn invalidate(&self) {
        let mut inner = self.inner.write().await;
        inner.epoch = inner.epoch.wrapping_add(1);
        if matches!(
            &inner.state,
            ThumbState::Ready(Source::External(_)) | ThumbState::Generating
        ) {
            return;
        }
        inner.state = ThumbState::Idle;
    }

    /// Ensures generation is underway (using `talk_service`) and reports status.
    pub(crate) async fn ensure(
        &self,
        talk_service: TalkService,
        mermaid: Arc<MermaidRenderer>,
        options: Arc<OverviewOptions>,
    ) -> ThumbStatus {
        if let Some(status) = self.snapshot().await {
            return status;
        }
        // Transition Idle -> Generating, re-checking under the write lock to
        // avoid spawning two generators on concurrent first requests. Capture the
        // epoch so the spawned task can detect a reload that happens mid-flight.
        let epoch = {
            let mut inner = self.inner.write().await;
            match &inner.state {
                ThumbState::Idle => {
                    inner.state = ThumbState::Generating;
                    inner.epoch
                }
                ThumbState::Generating => return ThumbStatus::Pending,
                ThumbState::Ready(_) => return ThumbStatus::Ready,
                ThumbState::Unavailable(reason) => {
                    return ThumbStatus::Unavailable(reason.clone());
                }
            }
        };
        self.spawn(talk_service, mermaid, options, epoch);
        ThumbStatus::Pending
    }

    /// Reads a generated asset (e.g. `overview.html`, `thumb-0001.png`) by its
    /// relative path, rejecting any path that tries to escape the cache
    /// directory.
    ///
    /// Distinguishes "not ready" from "ready but absent" so the route can send
    /// the browser back to `/slides` only in the first case: `/slides` redirects
    /// here whenever the overview is ready, so answering a ready-but-absent
    /// asset with that redirect would loop.
    pub(crate) async fn read_asset(&self, rel: &str) -> AssetLookup {
        if rel
            .split('/')
            .any(|segment| segment == ".." || segment.is_empty())
        {
            warn!(rel, "rejected overview asset path that escapes the cache");
            return AssetLookup::Missing;
        }
        let inner = self.inner.read().await;
        match &inner.state {
            ThumbState::Ready(source) => {
                let path = source.path().join(rel);
                match tokio::fs::read(&path).await {
                    Ok(bytes) => AssetLookup::Found(bytes),
                    // A genuinely absent asset is routine (the browser probing),
                    // but a permission or I/O error looks identical to the caller,
                    // so it is logged rather than silently flattened to a 404.
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => AssetLookup::Missing,
                    Err(err) => {
                        error!(path = %path.display(), "reading overview asset: {err}");
                        AssetLookup::Missing
                    }
                }
            }
            _ => AssetLookup::NotReady,
        }
    }

    /// Returns a status snapshot without transitioning out of `Idle`.
    async fn snapshot(&self) -> Option<ThumbStatus> {
        let inner = self.inner.read().await;
        match &inner.state {
            ThumbState::Ready(_) => Some(ThumbStatus::Ready),
            ThumbState::Generating => Some(ThumbStatus::Pending),
            ThumbState::Unavailable(reason) => Some(ThumbStatus::Unavailable(reason.clone())),
            ThumbState::Idle => None,
        }
    }

    fn spawn(
        &self,
        talk_service: TalkService,
        mermaid: Arc<MermaidRenderer>,
        options: Arc<OverviewOptions>,
        epoch: u64,
    ) {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            let talk = talk_service.source_talk().await;
            let result = generate(&talk, &mermaid, &options).await;

            let mut guard = inner.write().await;
            // Someone else owns the slot (a seeded external directory, say);
            // leave it entirely alone.
            if !matches!(&guard.state, ThumbState::Generating) {
                return;
            }
            // A reload (invalidate) bumps the epoch. Our result describes the
            // previous talk, so it is dropped — but the slot has to be handed
            // back, because `invalidate` no longer does it for us: it now leaves
            // an in-flight generation alone precisely so a second one cannot
            // start beside it. Releasing to `Idle` here is what lets the next
            // request regenerate against the current talk; returning without it
            // pinned the page on "generating…" for the life of the process.
            if guard.epoch != epoch {
                guard.state = ThumbState::Idle;
                return;
            }
            guard.state = match result {
                Ok(dir) => {
                    info!("generated slide-overview thumbnails");
                    ThumbState::Ready(Source::Owned(Arc::new(dir)))
                }
                Err(err) => {
                    error!("thumbnail generation failed: {err:?}");
                    ThumbState::Unavailable(unavailable_message(&err))
                }
            };
        });
    }
}

async fn generate(
    talk: &toboggan_core::Talk,
    mermaid: &MermaidRenderer,
    options: &OverviewOptions,
) -> anyhow::Result<TempDir> {
    let dir = tempfile::tempdir()?;
    generate_overview(talk, mermaid, dir.path(), options).await?;
    Ok(dir)
}

fn unavailable_message(err: &anyhow::Error) -> String {
    format!(
        "Slide thumbnails could not be generated (is a browser, or the `typst` binary, \
         installed?): {err}"
    )
}
