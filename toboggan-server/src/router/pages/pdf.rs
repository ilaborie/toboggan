use std::process::Command;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use toboggan_cli::OutputFormat;
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

    // Read the epoch before the talk: a reload in between then makes the epoch
    // stale, so the render is discarded rather than published as the current
    // deck's PDF. The reverse order could pair an old talk with a fresh epoch.
    let epoch = state.pdf_epoch().await;
    let talk = state.talk().await;
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
async fn render_pdf(talk: Talk) -> anyhow::Result<Vec<u8>> {
    let typst_source = toboggan_cli::output::serialize_talk(&talk, OutputFormat::Typst)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    tokio::task::spawn_blocking(move || compile_typst(&typst_source)).await?
}

fn compile_typst(source: &[u8]) -> anyhow::Result<Vec<u8>> {
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("talk.typ");
    let output = dir.path().join("talk.pdf");
    std::fs::write(&input, source)?;

    let status = Command::new("typst")
        .arg("compile")
        .arg(&input)
        .arg(&output)
        .status()
        .map_err(|err| anyhow::anyhow!("could not run `typst`: {err}"))?;
    if !status.success() {
        anyhow::bail!("`typst compile` failed with status {status}");
    }

    Ok(std::fs::read(&output)?)
}

fn slugify(title: &str) -> String {
    let slug = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "presentation".to_owned()
    } else {
        trimmed.to_owned()
    }
}
