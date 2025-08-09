//! Configuration and command-line argument parsing.
//!
//! This module defines the CLI interface and settings structure for the Toboggan CLI.

use std::path::PathBuf;

/// Command-line settings for the Toboggan CLI.
///
/// This structure defines all the command-line arguments and options that can be
/// passed to the CLI. It uses `clap` for parsing and validation.
///
/// # Examples
///
/// ```no_run
/// use clap::Parser;
/// use toboggan_cli::Settings;
///
/// // Parse from command line arguments
/// let settings = Settings::parse();
///
/// // Create manually for testing
/// let settings = Settings {
///     output: None,
///     title: Some("Test Talk".to_string()),
///     date: Some("2024-12-31".to_string()),
///     input: "slides.md".into(),
/// };
/// ```
#[derive(Debug, clap::Parser)]
#[command(
    name = "toboggan-cli",
    about = "Convert Markdown files and folders to Toboggan presentation TOML",
    long_about = "A command-line tool for creating Toboggan presentation configurations from Markdown files or structured folder hierarchies."
)]
pub struct Settings {
    /// Output file path for the generated TOML.
    ///
    /// If not specified, the output is written to stdout. This allows for easy
    /// piping and integration with other tools.
    #[clap(short, long, help = "Output file (default: stdout)")]
    pub output: Option<PathBuf>,

    /// Override the presentation title.
    ///
    /// This takes precedence over title.md, title.txt files, or folder names.
    /// Useful for batch processing or dynamic title generation.
    #[clap(short, long, help = "Title override (takes precedence over files)")]
    pub title: Option<String>,

    /// Override the presentation date.
    ///
    /// Must be in YYYY-MM-DD format. Takes precedence over date.txt files.
    /// If not specified, falls back to date.txt or today's date.
    #[clap(
        short,
        long,
        help = "Date override in YYYY-MM-DD format",
        value_parser = validate_date_format
    )]
    pub date: Option<String>,

    /// The input file or folder to process.
    ///
    /// Can be either:
    /// - A single Markdown file with slide separators
    /// - A folder containing structured presentation content
    #[clap(help = "Input file or folder to process")]
    pub input: PathBuf,
}

/// Validate date format for clap argument parsing.
fn validate_date_format(date_str: &str) -> Result<String, String> {
    use regex::Regex;

    let regex = Regex::new(r"^\d{4}-\d{1,2}-\d{1,2}$")
        .map_err(|error| format!("Internal regex error: {error}"))?;

    if regex.is_match(date_str) {
        Ok(date_str.to_string())
    } else {
        Err("Date must be in YYYY-MM-DD format (e.g., 2024-12-31)".to_string())
    }
}
