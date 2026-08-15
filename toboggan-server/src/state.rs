use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::FromRef;
use toboggan_core::{Command, Notification, Talk, Timestamp};
use tokio::sync::RwLock;

use crate::services::{AssetLookup, ClientService, TalkService, ThumbStatus, ThumbnailService};
use crate::{HealthResponse, HealthResponseStatus};

impl FromRef<TobogganState> for TalkService {
    fn from_ref(state: &TobogganState) -> Self {
        state.talk_service.clone()
    }
}

impl FromRef<TobogganState> for ClientService {
    fn from_ref(state: &TobogganState) -> Self {
        state.client_service.clone()
    }
}

/// A rendered PDF together with the download filename it was rendered under, so
/// a cache hit serves the same `Content-Disposition` as the render that filled
/// it.
#[derive(Clone)]
pub(crate) struct CachedPdf {
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) slug: Arc<str>,
}

/// The rendered-PDF cache plus an `epoch` that is bumped on every reload.
///
/// A render captures the epoch before it reads the talk and only commits if the
/// epoch is still current, so a reload landing mid-render cannot have its
/// invalidation undone by the in-flight (now stale) result. Mirrors the same
/// guard in [`ThumbnailService`].
#[derive(Default)]
struct PdfCache {
    epoch: u64,
    rendered: Option<CachedPdf>,
}

/// Thin coordinator/facade that orchestrates `TalkService` and `ClientService`
#[derive(Clone)]
pub struct TobogganState {
    started_at: Timestamp,
    talk_service: TalkService,
    client_service: ClientService,
    /// Shell spawned for embedded terminals. `Arc<str>` keeps state clones cheap,
    /// since the whole state is cloned on every request extraction.
    terminal_shell: Arc<str>,
    /// Lazily-rendered PDF of the current talk, invalidated on reload.
    pdf_cache: Arc<RwLock<PdfCache>>,
    /// Lazily-generated slide-overview thumbnails, invalidated on reload.
    thumbnail_service: ThumbnailService,
}

impl TobogganState {
    /// Creates a new `TobogganState` with the given services and embedded-terminal shell
    #[must_use]
    pub fn new(
        talk_service: TalkService,
        client_service: ClientService,
        terminal_shell: Arc<str>,
    ) -> Self {
        Self {
            started_at: Timestamp::now(),
            talk_service,
            client_service,
            terminal_shell,
            pdf_cache: Arc::new(RwLock::new(PdfCache::default())),
            thumbnail_service: ThumbnailService::new(),
        }
    }

    /// Returns a clone of the current talk.
    pub(crate) async fn talk(&self) -> Talk {
        self.talk_service.talk().await
    }

    /// Returns the cached rendered PDF, if any.
    pub(crate) async fn cached_pdf(&self) -> Option<CachedPdf> {
        self.pdf_cache.read().await.rendered.clone()
    }

    /// The current PDF epoch. Capture this *before* reading the talk to render,
    /// so any reload that races the render is guaranteed to invalidate it.
    pub(crate) async fn pdf_epoch(&self) -> u64 {
        self.pdf_cache.read().await.epoch
    }

    /// Stores a rendered PDF, unless a reload has happened since `epoch` was
    /// captured — in which case the render is of a talk that no longer exists
    /// and is dropped rather than published.
    pub(crate) async fn store_pdf(&self, epoch: u64, pdf: CachedPdf) {
        let mut cache = self.pdf_cache.write().await;
        if cache.epoch == epoch {
            cache.rendered = Some(pdf);
        }
    }

    /// Pre-seeds an externally-generated thumbnails directory (`--thumbnails-dir`).
    pub(crate) async fn seed_thumbnails_dir(&self, dir: PathBuf) {
        self.thumbnail_service.seed_external(dir).await;
    }

    /// Ensures slide-overview thumbnails are being generated and reports status.
    pub(crate) async fn ensure_thumbnails(&self) -> ThumbStatus {
        self.thumbnail_service
            .ensure(self.talk_service.clone())
            .await
    }

    /// Reads a generated overview asset by its relative path.
    pub(crate) async fn thumbnail_asset(&self, rel: &str) -> AssetLookup {
        self.thumbnail_service.read_asset(rel).await
    }

    /// Returns the shell to spawn for embedded terminals
    #[must_use]
    pub(crate) fn terminal_shell(&self) -> &str {
        &self.terminal_shell
    }

    /// Returns the health status of the server
    pub(crate) async fn health(&self) -> HealthResponse {
        let status = HealthResponseStatus::Ok;
        let started_at = self.started_at;
        let elapsed = started_at.elapsed();
        let talk = self.talk_service.title().await;
        let active_clients = self.client_service.active_clients_count().await;

        HealthResponse {
            status,
            started_at,
            elapsed,
            talk,
            active_clients,
        }
    }

    /// Handles a command, broadcasts the notification to all clients, and returns it
    pub async fn handle_command(&self, command: &Command) -> Notification {
        let start_time = std::time::Instant::now();

        let notification = self.talk_service.handle_command(command).await;
        self.client_service.notify_all(&notification).await;

        let active_clients = self.client_service.active_clients_count().await;
        tracing::debug!(
            ?command,
            duration_ms = start_time.elapsed().as_millis(),
            active_clients,
            "Command handled and broadcast completed"
        );

        notification
    }

    /// Reloads the talk and broadcasts the change to all clients
    ///
    /// # Errors
    /// Returns an error if the new talk has no slides
    pub async fn reload_talk(&self, new_talk: Talk) -> anyhow::Result<()> {
        let notification = self.talk_service.reload_talk(new_talk).await?;
        // The cached PDF and thumbnails are now stale. Bumping the epoch also
        // tells an in-flight render to discard its result instead of re-filling
        // the cache we are clearing here.
        {
            let mut cache = self.pdf_cache.write().await;
            cache.rendered = None;
            cache.epoch = cache.epoch.wrapping_add(1);
        }
        self.thumbnail_service.invalidate().await;
        self.client_service.notify_all(&notification).await;
        Ok(())
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
