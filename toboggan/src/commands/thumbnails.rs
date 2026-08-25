use std::path::{Path, PathBuf};

use toboggan_cli::output::{ThumbnailOptions, generate_thumbnails};

/// Generates per-slide thumbnails, a search index, and a self-contained overview
/// page for a presentation folder.
///
/// # Errors
/// Returns an error if the folder cannot be parsed or thumbnail rendering fails
/// (e.g. the `typst` binary is missing).
#[allow(clippy::print_stdout)]
pub(crate) fn generate(
    input: &Path,
    mut settings: toboggan_cli::Settings,
    output: Option<PathBuf>,
    search: bool,
    static_links: bool,
) -> anyhow::Result<()> {
    super::ensure_typst()?;

    let slides = super::deck::resolve_deck(input).slides;
    settings.input = Some(slides.clone());
    let talk = super::deck::build_talk(&slides, &settings)?;

    let out_dir = output.unwrap_or_else(|| PathBuf::from("overview"));
    let options = ThumbnailOptions {
        search,
        static_links,
        ..ThumbnailOptions::new(toboggan_cli::mermaid_renderer(&settings)?)
    };
    generate_thumbnails(&talk, &out_dir, &options).map_err(|err| anyhow::anyhow!("{err}"))?;

    println!(
        "✅ Wrote {} thumbnails + overview.html to {}",
        talk.slides.len(),
        out_dir.display()
    );
    Ok(())
}
