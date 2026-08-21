use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::FromRef;
use toboggan_core::{Command, Notification, Talk, Timestamp};
use tokio::sync::RwLock;

use crate::services::{AssetLookup, ClientService, TalkService, ThumbStatus, ThumbnailService};
use crate::{HealthResponse, HealthResponseStatus, PresenterAuth};

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

impl FromRef<TobogganState> for PresenterAuth {
    fn from_ref(state: &TobogganState) -> Self {
        state.auth.clone()
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
    /// Held for the duration of a PDF render so only one runs at a time.
    pdf_render_lock: Arc<tokio::sync::Mutex<()>>,
    /// Lazily-generated slide-overview thumbnails, invalidated on reload.
    thumbnail_service: ThumbnailService,
    /// Decides which connections may drive the deck and open terminals.
    auth: PresenterAuth,
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
            pdf_render_lock: Arc::new(tokio::sync::Mutex::new(())),
            thumbnail_service: ThumbnailService::new(),
            auth: PresenterAuth::default(),
        }
    }

    /// Installs the presenter gate.
    ///
    /// Separate from [`Self::new`] so the default is the closed-but-quiet one:
    /// a state built without thinking about it grants the presenter role to
    /// this machine only, which is exactly what a server bound to loopback
    /// wants.
    #[must_use]
    pub fn with_auth(mut self, auth: PresenterAuth) -> Self {
        self.auth = auth;
        self
    }

    /// The role a connection from `peer` offering `token` is granted.
    pub(crate) fn role_for(
        &self,
        peer: std::net::IpAddr,
        token: Option<&str>,
    ) -> toboggan_core::ClientRole {
        self.auth.role_for(peer, token)
    }

    /// Returns the cached rendered PDF, if any.
    pub(crate) async fn cached_pdf(&self) -> Option<CachedPdf> {
        self.pdf_cache.read().await.rendered.clone()
    }

    /// The inputs for one PDF render: the epoch to commit under, and the talk.
    ///
    /// Returned together so the ordering cannot be got wrong. The epoch must be
    /// read *before* the talk: a reload in between then leaves the epoch stale
    /// and the render is discarded, whereas the reverse order would pair an old
    /// talk with a fresh epoch and cache a stale PDF as current.
    pub(crate) async fn pdf_render_input(&self) -> (u64, Arc<Talk>) {
        let epoch = self.pdf_cache.read().await.epoch;
        (epoch, self.talk_service.source_talk().await)
    }

    /// Waits for the right to render the PDF.
    ///
    /// `typst` compilation is expensive and runs on the blocking pool, so
    /// without this every request arriving before the first render committed
    /// started its own — 30 cold requests meant 30 `typst` children and 30 full
    /// PDFs in memory. Holders re-check the cache after acquiring: the usual
    /// outcome is that the first renderer filled it while they waited.
    pub(crate) async fn pdf_render_permit(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.pdf_render_lock.lock().await
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
