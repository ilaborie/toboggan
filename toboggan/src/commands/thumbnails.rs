use std::path::PathBuf;

use toboggan_cli::output::{ThumbnailOptions, generate_thumbnails};

use crate::cli::ThumbnailsArgs;

/// Generates per-slide thumbnails, a search index, and a self-contained overview
/// page for a presentation folder.
///
/// # Errors
/// Returns an error if the folder cannot be parsed or thumbnail rendering fails
/// (e.g. the `typst` binary is missing).
#[allow(clippy::print_stdout)]
pub(crate) fn generate(args: ThumbnailsArgs) -> anyhow::Result<()> {
    let ThumbnailsArgs {
        input,
        output,
        no_search,
        build,
    } = args;

    super::ensure_typst()?;

    let settings = build.into_cli_settings(input.clone(), true);
    let parse_result = toboggan_cli::parse_presentation(&input, &settings)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let talk = parse_result.to_talk();

    let out_dir = output.unwrap_or_else(|| PathBuf::from("overview"));
    let options = ThumbnailOptions { search: !no_search };
    generate_thumbnails(&talk, &out_dir, options).map_err(|err| anyhow::anyhow!("{err}"))?;

    println!(
        "✅ Wrote {} thumbnails + overview.html to {}",
        talk.slides.len(),
        out_dir.display()
    );
    Ok(())
}
