use crate::cli::{OpenapiArgs, StatsArgs};

/// Prints presentation statistics for a folder.
///
/// # Errors
/// Returns an error if the folder cannot be parsed or stats cannot be written.
pub(crate) fn show_stats(args: StatsArgs) -> anyhow::Result<()> {
    let mut settings = toboggan_cli::Settings::from(args);
    let input = settings
        .input
        .clone()
        .ok_or_else(|| anyhow::anyhow!("missing input folder"))?;
    let slides = super::deck::resolve_deck(&input).slides;
    settings.input = Some(slides.clone());

    let parse_result = toboggan_cli::parse_presentation(&slides, &settings)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    for error in parse_result.errors() {
        tracing::warn!("slide excluded from the stats: {error}");
    }
    let stats = toboggan_cli::stats::PresentationStats::from_parse_result(
        &parse_result,
        settings.wpm,
        !settings.exclude_notes_from_duration,
    );
    stats
        .display(
            &mut std::io::stdout(),
            toboggan_cli::display::DisplayConfig::should_use_colors(),
        )
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    Ok(())
}

/// Emits the bundled `OpenAPI` document to a file or stdout.
///
/// # Errors
/// Returns an error if the output file cannot be written.
#[allow(clippy::print_stdout)]
pub(crate) fn emit_openapi(args: OpenapiArgs) -> anyhow::Result<()> {
    let json = toboggan_server::openapi_json();
    match args.output {
        Some(path) => {
            std::fs::write(&path, json)?;
            tracing::info!(path = %path.display(), "wrote OpenAPI document");
        }
        None => println!("{json}"),
    }
    Ok(())
}
