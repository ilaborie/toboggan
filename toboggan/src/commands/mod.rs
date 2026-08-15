pub(crate) mod build_serve;
pub(crate) mod deck;
pub(crate) mod lint;
pub(crate) mod misc;
pub(crate) mod new;
pub(crate) mod pdf;
pub(crate) mod skills;
pub(crate) mod thumbnails;

/// Fails fast with a friendly message when the `typst` binary is not on `PATH`.
///
/// Both `pdf` and `thumbnails` shell out to `typst`; probing it up front turns a
/// late mid-render failure into an immediate, actionable error.
///
/// # Errors
/// Returns an error if `typst --version` cannot be spawned (binary missing).
pub(crate) fn ensure_typst() -> anyhow::Result<()> {
    std::process::Command::new("typst")
        .arg("--version")
        .output()
        .map_err(|err| {
            anyhow::anyhow!(
                "`typst` was not found on PATH ({err}); install it from https://typst.app to render PDFs and thumbnails"
            )
        })?;
    Ok(())
}
