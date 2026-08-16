use std::path::{Path, PathBuf};
use std::process::Command;

use toboggan_cli::OutputFormat;

/// Builds a PDF from a folder by rendering Typst, then shelling out to `typst`.
///
/// # Errors
/// Returns an error if parsing fails, the `.typ` cannot be written, the `typst`
/// binary is missing, or compilation fails.
#[allow(clippy::print_stdout)]
pub(crate) fn build_pdf(
    input: &Path,
    mut settings: toboggan_cli::Settings,
    output: Option<PathBuf>,
) -> anyhow::Result<()> {
    super::ensure_typst()?;

    let deck = super::deck::resolve_deck(input);
    settings.input = Some(deck.slides.clone());
    let output = output.unwrap_or_else(|| default_pdf_path(&deck.slides));

    let talk = super::deck::build_talk(&deck.slides, &settings)?;
    let typst_source = toboggan_cli::output::serialize_talk(&talk, OutputFormat::Typst)
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    // Two things have to line up for a slide's `#image("../public/logo.png")` to
    // resolve: the intermediate `.typ` must sit where the slides do (typst
    // resolves relative paths against the *file*), and the project root must be
    // the deck (typst refuses any path that escapes it). Compiling a temp file
    // from elsewhere failed every such slide with "would escape the project root".
    let root = deck.root();
    let typ_path = deck.slides.join(".toboggan-pdf.typ");
    std::fs::write(&typ_path, &typst_source)
        .map_err(|err| anyhow::anyhow!("writing {}: {err}", typ_path.display()))?;

    let status = Command::new("typst")
        .arg("compile")
        .arg("--root")
        .arg(&root)
        .arg(&typ_path)
        .arg(&output)
        .status()
        .map_err(|err| {
            anyhow::anyhow!("could not run `typst` (is it installed and on PATH?): {err}")
        })?;

    // Best-effort: the PDF is the deliverable, and leaving the scratch file
    // behind on success was a reported annoyance.
    if let Err(err) = std::fs::remove_file(&typ_path) {
        tracing::debug!("could not remove {}: {err}", typ_path.display());
    }

    if !status.success() {
        anyhow::bail!("`typst compile` failed with status {status}");
    }

    println!("✅ Wrote {}", output.display());
    Ok(())
}

/// Derives a default `<name>.pdf` path from the input folder, treating a
/// trailing `slides/` as the deck's `slides` directory.
fn default_pdf_path(input: &Path) -> PathBuf {
    let name = input
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| *name != "slides")
        .or_else(|| {
            input
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
        })
        .unwrap_or("presentation");
    PathBuf::from(format!("{name}.pdf"))
}
