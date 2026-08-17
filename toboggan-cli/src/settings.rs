use std::path::PathBuf;

use toboggan_core::Date;

use crate::parse_date_string;

/// Output format for the generated presentation
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    /// TOML format (default)
    Toml,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Static HTML file (single file with inlined CSS)
    Html,
    /// Typst file for PDF compilation (`typst compile output.typ`)
    Typst,
}

/// Command-line settings for the Toboggan CLI.
#[derive(Debug, Clone, clap::Parser)]
#[command(
    name = "toboggan-cli",
    about = "Convert Markdown folders to Toboggan presentation TOML",
    long_about = "A command-line tool for creating Toboggan presentation configurations from structured folder hierarchies containing Markdown and HTML files."
)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    /// Output file path for the generated presentation.
    ///
    /// Required to write the deck: with no `-o`, the run prints a tip and writes
    /// nothing — there is no stdout fallback. The file extension drives the
    /// default format (`.toml`, `.json`, `.yaml`, `.html`, `.typ`).
    #[clap(short, long, help = "Output file (required to write the deck)")]
    pub output: Option<PathBuf>,

    /// Override the presentation title.
    ///
    /// This takes precedence over front matter title in _cover.md or folder names.
    /// Useful for batch processing or dynamic title generation.
    #[clap(
        short,
        long,
        help = "Title override (takes precedence over frontmatter)"
    )]
    pub title: Option<String>,

    /// Override the presentation date.
    ///
    /// Must be in YYYY-MM-DD format. Takes precedence over front matter date in _cover.md.
    /// If not specified, falls back to front matter date or today's date.
    #[clap(
        short,
        long,
        help = "Date override in YYYY-MM-DD format",
        value_parser = parse_date_string
    )]
    pub date: Option<Date>,

    /// BCP 47 language tag for the deck, e.g. `fr`.
    ///
    /// Takes precedence over the `lang` in `_cover.md` front matter. Becomes the
    /// `lang` attribute on every page Toboggan renders.
    #[clap(
        long,
        help = "Deck language tag (e.g. fr); overrides the cover frontmatter"
    )]
    pub lang: Option<String>,

    /// Base URL the exported HTML will be served from.
    ///
    /// Only meaningful for the HTML export, where it resolves the deck's
    /// `public/` assets. Empty — the default — leaves them relative to the
    /// exported file, which is right whenever `public/` travels with it. Set it
    /// for a deploy that serves the file from a known path, e.g. `/my-talk/`.
    #[clap(
        long,
        help = "Base URL the exported HTML is served from (e.g. /my-talk/)"
    )]
    pub base_url: Option<String>,

    /// Syntax highlighting theme for code blocks.
    ///
    /// Available themes: `base16-ocean.dark`, `base16-ocean.light`, `base16-mocha.dark`,
    /// `base16-eighties.dark`, `InspiredGitHub`, `Solarized (dark)`, `Solarized (light)`,
    /// `Monokai`, `Monokai Extended`, `Monokai Extended Light`, `Monokai Extended Bright`,
    /// and many more from the syntect library.
    ///
    /// Use `--list-themes` to see all available themes.
    #[clap(
        long,
        default_value = "base16-ocean.light",
        help = "Syntax highlighting theme (default: base16-ocean.light)"
    )]
    pub theme: String,

    /// List all available syntax highlighting themes and exit.
    #[clap(long, help = "List all available syntax highlighting themes and exit")]
    pub list_themes: bool,

    /// Output format for the generated presentation.
    #[clap(
        short = 'f',
        long,
        help = "Output format (toml, json, yaml, html, typst). Auto-detected from the output file extension when omitted."
    )]
    pub format: Option<OutputFormat>,

    /// Disable automatic numbering of parts and slides.
    ///
    /// By default, parts are numbered (1., 2., etc.) and slides within parts
    /// are numbered (1.1, 1.2, etc.). This flag disables that behavior.
    #[clap(long, help = "Disable automatic numbering of parts and slides")]
    pub no_counter: bool,

    /// Disable presentation statistics display.
    ///
    /// By default, comprehensive statistics are shown including word count,
    /// duration estimates, and part breakdown. This flag disables that output.
    #[clap(long, help = "Disable presentation statistics display")]
    pub no_stats: bool,

    /// Set speaking rate in words per minute for duration estimates.
    ///
    /// Used to calculate presentation duration. Typical rates:
    /// - Slow: 110 WPM
    /// - Normal: 150 WPM (default)
    /// - Fast: 170 WPM
    #[clap(
        long,
        default_value = "150",
        help = "Speaking rate in words per minute (default: 150)"
    )]
    pub wpm: u16,

    /// Exclude speaker notes from duration calculations.
    ///
    /// By default, words in speaker notes are counted toward total duration.
    /// This flag excludes notes from duration calculations.
    #[clap(long, help = "Exclude speaker notes from duration calculations")]
    pub exclude_notes_from_duration: bool,

    /// The input folder to process.
    ///
    /// Must be a folder containing structured presentation content.
    /// The folder should contain markdown (.md) and/or HTML (.html) files.
    /// Custom head HTML can be provided via a `_head.html` file in the input folder.
    #[clap(help = "Input folder to process")]
    pub input: Option<PathBuf>,
}

impl Settings {
    /// Determine the output format, auto-detecting from file extension if not specified
    #[must_use]
    pub fn resolve_format(&self) -> OutputFormat {
        // If format is explicitly specified, use it
        if let Some(format) = self.format {
            return format;
        }

        // Try to auto-detect from output file extension
        if let Some(output_path) = &self.output
            && let Some(extension) = output_path.extension().and_then(|ext| ext.to_str())
        {
            match extension.to_lowercase().as_str() {
                "toml" => return OutputFormat::Toml,
                "json" => return OutputFormat::Json,
                "yaml" | "yml" => return OutputFormat::Yaml,
                "html" | "htm" => return OutputFormat::Html,
                "typ" => return OutputFormat::Typst,
                // An unrecognised extension used to fall through to TOML, so
                // `-o out.pdf` wrote TOML into a file named `.pdf` and exited 0.
                // Warn rather than fail: `-f` may still name the format, and the
                // extension is only ever a hint.
                other => tracing::warn!(
                    extension = other,
                    "unrecognised output extension; defaulting to TOML (use -f to choose a format)"
                ),
            }
        }

        // Default to TOML if no format specified and can't auto-detect
        OutputFormat::Toml
    }
}
