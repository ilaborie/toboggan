pub(crate) mod build_serve;
pub(crate) mod ci;
pub(crate) mod deck;
pub(crate) mod lint;
pub(crate) mod misc;
pub(crate) mod new;
pub(crate) mod pdf;
pub(crate) mod skills;
pub(crate) mod thumbnails;

use std::path::{Path, PathBuf};

/// Version-control directories that mark the root of a repository.
///
/// Both, because `jj git init` without `--colocate` leaves no top-level `.git`:
/// a deck under Jujutsu is still a repository that pushes to GitHub, so looking
/// only for `.git` would find no root and put the workflow in the wrong place.
pub(crate) const VCS_MARKERS: [&str; 2] = [".jj", ".git"];

/// The root of the repository containing `dir` — the nearest ancestor holding
/// one of `markers` — or `None` when `dir` is not inside one.
///
/// Canonicalize first: `Path::parent` on a relative `"mytalk"` yields `""` and
/// then `None`, so the walk stopped at the directory itself and never saw an
/// enclosing repo. `toboggan new mytalk` from inside a checkout would then
/// initialize a nested repository, while the same command with an absolute path
/// correctly skipped.
pub(crate) fn repo_root(dir: &Path, markers: &[&str]) -> Option<PathBuf> {
    let canonical = dir.canonicalize();
    let start = canonical.as_deref().unwrap_or(dir);
    let mut current = Some(start);
    while let Some(path) = current {
        if markers.iter().any(|marker| path.join(marker).exists()) {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

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
