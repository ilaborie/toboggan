use std::path::{Path, PathBuf};
use std::process::Command;

use toboggan_cli::OutputFormat;

use crate::cli::PdfArgs;

/// Builds a PDF from a folder by rendering Typst, then shelling out to `typst`.
///
/// # Errors
/// Returns an error if parsing fails, the `.typ` cannot be written, the `typst`
/// binary is missing, or compilation fails.
#[allow(clippy::print_stdout)]
pub(crate) fn build_pdf(args: PdfArgs) -> anyhow::Result<()> {
    let settings = args.cli_settings();
    let PdfArgs { input, output, .. } = args;
    let output = output.unwrap_or_else(|| default_pdf_path(&input));

    let parse_result = toboggan_cli::parse_presentation(&input, &settings)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let talk = parse_result.to_talk();
    let typst_source = toboggan_cli::output::serialize_talk(&talk, OutputFormat::Typst)
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    let typ_path = output.with_extension("typ");
    std::fs::write(&typ_path, &typst_source)
        .map_err(|err| anyhow::anyhow!("writing {}: {err}", typ_path.display()))?;

    let status = Command::new("typst")
        .arg("compile")
        .arg(&typ_path)
        .arg(&output)
        .status()
        .map_err(|err| {
            anyhow::anyhow!("could not run `typst` (is it installed and on PATH?): {err}")
        })?;

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
