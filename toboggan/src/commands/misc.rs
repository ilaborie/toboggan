use crate::cli::OpenapiArgs;

/// Prints presentation statistics for a folder.
///
/// # Errors
/// Returns an error if the folder cannot be parsed or stats cannot be written.
pub(crate) fn show_stats(mut settings: toboggan_cli::Settings) -> anyhow::Result<()> {
    let input = settings
        .input
        .clone()
        .ok_or_else(|| anyhow::anyhow!("missing input folder"))?;
    let slides = super::deck::resolve_deck(&input).slides;
    settings.input = Some(slides.clone());

    let mut parse_result =
        toboggan_cli::parse_presentation(&slides, &settings).map_err(anyhow::Error::new)?;
    // Refuse rather than warn. Word counts and a duration estimate over a deck
    // that is quietly missing its unparseable slides are worse than no numbers
    // at all: a rehearsal timed against them is wrong, and the warning that
    // used to be the only clue is filtered out by default.
    let failures = parse_result.take_errors();
    if !failures.is_empty() {
        return Err(anyhow::Error::new(
            toboggan_cli::TobogganCliError::SlidesFailedToParse { failures },
        ));
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
