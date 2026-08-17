use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use toboggan_cli::OutputFormat;
use toboggan_cli::scaffold::slugify;
use toboggan_core::Talk;
use tracing::{error, info};

use crate::TobogganState;
use crate::state::CachedPdf;

/// Serves the talk as a downloadable PDF at `/download.pdf`.
///
/// The PDF is rendered on demand via the `typst` binary and cached until the
/// talk reloads. If `typst` is missing or compilation fails, returns `503`.
pub(crate) async fn download_pdf(State(state): State<TobogganState>) -> Response {
    if let Some(cached) = state.cached_pdf().await {
        return pdf_response(&cached.bytes, &cached.slug);
    }

    // Only one render at a time; everyone else waits here rather than starting
    // their own `typst` process.
    let _permit = state.pdf_render_permit().await;
    // Re-check: the render we queued behind has almost certainly filled the cache.
    if let Some(cached) = state.cached_pdf().await {
        return pdf_response(&cached.bytes, &cached.slug);
    }

    let (epoch, talk) = state.pdf_render_input().await;
    let slug: Arc<str> = Arc::from(slugify(&talk.title));
    match render_pdf(talk).await {
        Ok(bytes) => {
            let bytes: Arc<[u8]> = Arc::from(bytes);
            let cached = CachedPdf {
                bytes: Arc::clone(&bytes),
                slug: Arc::clone(&slug),
            };
            state.store_pdf(epoch, cached).await;
            info!(bytes = bytes.len(), "rendered PDF");
            pdf_response(&bytes, &slug)
        }
        Err(err) => {
            error!("PDF generation failed: {err:?}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("PDF generation failed (is `typst` installed?): {err}"),
            )
                .into_response()
        }
    }
}

fn pdf_response(bytes: &[u8], slug: &str) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{slug}.pdf\""),
            ),
        ],
        bytes.to_vec(),
    )
        .into_response()
}

/// Renders the talk to Typst, then compiles it to PDF in a blocking task.
async fn render_pdf(talk: Arc<Talk>) -> anyhow::Result<Vec<u8>> {
    let typst_source = toboggan_cli::output::serialize_talk(&talk, OutputFormat::Typst)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    // Compile with the deck as the project root so slides that reference
    // `../public/...` images resolve; without it typst rejects each one and the
    // whole download fails with a bare exit status.
    let root = toboggan_cli::output::deck_root(&talk);
    let slides = talk.source_dir.as_deref().map(PathBuf::from);
    tokio::task::spawn_blocking(move || {
        compile_typst(&typst_source, root.as_deref(), slides.as_deref())
    })
    .await?
}

fn compile_typst(
    source: &[u8],
    root: Option<&Path>,
    slides: Option<&Path>,
) -> anyhow::Result<Vec<u8>> {
    let dir = tempfile::tempdir()?;
    // Beside the slides when the deck came from a folder, so a slide's relative
    // `#image("../public/…")` resolves as authored; a temp dir otherwise.
    let input = slides.map_or_else(
        || dir.path().join("talk.typ"),
        |slides| slides.join(".toboggan-download.typ"),
    );
    let output = dir.path().join("talk.pdf");
    std::fs::write(&input, source)?;

    let mut command = Command::new("typst");
    command.arg("compile");
    if let Some(root) = root {
        command.arg("--root").arg(root);
    }
    let result = command
        .arg(&input)
        .arg(&output)
        .output()
        .map_err(|err| anyhow::anyhow!("could not run `typst`: {err}"))?;
    cleanup_input(&input, slides);
    if !result.status.success() {
        // Include typst's own diagnostics: a bare exit status left the user with
        // nothing to act on.
        let stderr = String::from_utf8_lossy(&result.stderr);
        anyhow::bail!(
            "`typst compile` failed ({}): {}",
            result.status,
            stderr.trim()
        );
    }

    Ok(std::fs::read(&output)?)
}

/// Removes the intermediate `.typ` when it was written into the deck.
fn cleanup_input(input: &Path, slides: Option<&Path>) {
    if slides.is_some()
        && let Err(err) = std::fs::remove_file(input)
    {
        tracing::debug!("could not remove {}: {err}", input.display());
    }
}
