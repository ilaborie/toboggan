//! # Toboggan CLI
//!
//! A command-line interface for creating and converting Toboggan presentation files.
//! This tool can parse both flat Markdown files and structured folder hierarchies
//! to generate TOML configuration files for presentations.
//!
//! ## Overview
//!
//! The Toboggan CLI provides a flexible way to create presentation configurations:
//!
//! - **Flat Markdown**: Convert single Markdown files with slide separators
//! - **Folder Structure**: Process organized folder hierarchies with automatic slide detection
//! - **CLI Overrides**: Override title and date via command-line arguments
//! - **File Fallbacks**: Use title.md/title.txt and date.txt files for metadata
//!
//! ## Usage
//!
//! ```bash
//! # Convert a Markdown file to TOML
//! toboggan-cli presentation.md -o talk.toml
//!
//! # Process a folder structure
//! toboggan-cli slides-folder/ -o talk.toml
//!
//! # Override title and date
//! toboggan-cli slides/ --title "My Talk" --date "2024-12-31"
//!
//! # Output to stdout (for piping)
//! toboggan-cli presentation.md
//! ```
//!
//! ## Input Formats
//!
//! ### Flat Markdown Files
//!
//! Flat Markdown files use horizontal rules (`---`) to separate slides and heading levels
//! to determine slide types:
//!
//! ```markdown
//! # Presentation Title
//!
//! ## First Section
//! Content for section slide
//!
//! ---
//!
//! ### Regular Slide
//! Regular slide content
//!
//! > Speaker notes go in blockquotes
//!
//! ---
//!
//! ### Another Slide
//! More content
//! ```
//!
//! - H1 (`#`) creates the presentation title
//! - H2 (`##`) creates Part slides (section dividers)
//! - H3+ (`###`) creates Standard slides
//! - Blockquotes (`>`) become speaker notes
//! - Horizontal rules (`---`) separate slides
//!
//! ### Folder Structure
//!
//! For complex presentations, organize content in folders:
//!
//! ```text
//! my-talk/
//! ├── title.md              # Presentation title (optional)
//! ├── date.txt              # Presentation date (optional)
//! ├── _cover.md             # Cover slide
//! ├── _footer.md            # Footer content
//! ├── 01-intro/             # Section folder
//! │   ├── _part.md          # Section slide
//! │   ├── 01-overview.md    # Content slides
//! │   └── 02-goals.md
//! ├── 02-content/
//! │   ├── _part.md
//! │   ├── slide1.md
//! │   └── slide2.html       # HTML slides supported
//! └── 99-conclusion.md      # Final slide
//! ```
//!
//! #### Special Files
//!
//! - **`title.md`/`title.txt`**: Presentation title (fallback to folder name)
//! - **`date.txt`**: Presentation date in YYYY-MM-DD format (fallback to today)
//! - **`_cover.md`**: Cover slide (created first)
//! - **`_part.md`**: Section divider slide content
//! - **`_footer.md`**: Presentation footer (markdown content)
//!
//! #### Processing Rules
//!
//! 1. Files are processed in alphabetical order
//! 2. Folders become Part slides with contents as subsequent slides
//! 3. Both `.md` and `.html` files are supported
//! 4. Hidden files (starting with `.`) are ignored
//! 5. Markdown files are converted to HTML with original markdown as alt text
//! 6. HTML files are used directly
//!
//! ## Command-Line Options
//!
//! ```text
//! toboggan-cli [OPTIONS] <INPUT>
//!
//! Arguments:
//!   <INPUT>  The input file or folder to process
//!
//! Options:
//!   -o, --output <OUTPUT>  Output file (default: stdout)
//!   -t, --title <TITLE>    Title override (takes precedence over files)
//!   -d, --date <DATE>      Date override in YYYY-MM-DD format
//!   -h, --help             Print help
//! ```
//!
//! ## Output Format
//!
//! The tool generates TOML files compatible with the Toboggan presentation system:
//!
//! ```toml
//! date = "2024-01-26"
//!
//! [title]
//! type = "Text"
//! text = "My Presentation"
//!
//! [[slides]]
//! kind = "Cover"
//! style = []
//!
//! [slides.title]
//! type = "Text"
//! text = "Welcome"
//!
//! [slides.body]
//! type = "Html"
//! raw = "<p>Welcome to my presentation</p>"
//! alt = "Welcome to my presentation"
//!
//! [slides.notes]
//! type = "Text"
//! text = "Remember to speak slowly"
//! ```
//!
//! ## Examples
//!
//! ### Creating a Simple Presentation
//!
//! ```bash
//! # Create a simple markdown file
//! echo "# My Talk
//!
//! ## Introduction
//! Welcome to my presentation
//!
//! ---
//!
//! ### Key Points
//! - Point 1
//! - Point 2
//!
//! > Don't forget to mention the demo" > slides.md
//!
//! # Convert to TOML
//! toboggan-cli slides.md -o presentation.toml
//! ```
//!
//! ### Working with Folders
//!
//! ```bash
//! mkdir -p my-talk/01-intro
//! echo "My Amazing Talk" > my-talk/title.md
//! echo "2024-03-15" > my-talk/date.txt
//! echo "# Welcome" > my-talk/_cover.md
//! echo "## Chapter 1" > my-talk/01-intro/_part.md
//! echo "### Overview" > my-talk/01-intro/overview.md
//!
//! toboggan-cli my-talk/ -o talk.toml
//! ```
//!
//! ### Dynamic Title and Date
//!
//! ```bash
//! # Use current date and custom title
//! toboggan-cli slides/ --title "$(date '+%Y Conference Talk')" --date "$(date '+%Y-%m-%d')"
//! ```
//!
//! ## Integration
//!
//! The CLI integrates seamlessly with build systems and automation:
//!
//! ```bash
//! # Build pipeline example
//! for dir in talks/*/; do
//!   talk_name=$(basename "$dir")
//!   toboggan-cli "$dir" -o "output/${talk_name}.toml"
//! done
//!
//! # With custom metadata
//! find presentations/ -name "*.md" -exec toboggan-cli {} --date "$(date '+%Y-%m-%d')" \;
//! ```

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, bail};
use toboggan_core::{Content, Date};
use tracing::info;

mod parser;
mod settings;

use parser::{FlatFileParser, FolderParser, Parser};

pub use self::settings::*;

/// Launch the CLI with the given settings.
///
/// This is the main entry point for the CLI application. It processes the input
/// (either a file or folder) according to the settings and outputs the resulting
/// TOML configuration.
///
/// # Arguments
///
/// * `settings` - CLI settings containing input path, output options, and overrides
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if parsing or writing fails.
///
/// # Example
///
/// ```no_run
/// use toboggan_cli::{Settings, launch};
/// use std::path::PathBuf;
///
/// let settings = Settings {
///     output: Some(PathBuf::from("output.toml")),
///     title: Some("My Talk".to_string()),
///     date: Some("2024-12-31".to_string()),
///     input: PathBuf::from("slides/"),
/// };
///
/// launch(settings).expect("Failed to process presentation");
/// ```
#[doc(hidden)]
#[allow(clippy::print_stderr)]
pub fn launch(settings: Settings) -> anyhow::Result<()> {
    info!(?settings, "launching CLI...");
    let Settings {
        output,
        input,
        title,
        date,
    } = settings;

    // Parse title from CLI argument if provided
    let title_override = title.map(Content::from);

    // Parse date from CLI argument if provided
    let date_override = if let Some(date_str) = date {
        Some(parse_date_string(&date_str).with_context(|| format!("parsing date '{date_str}"))?)
    } else {
        None
    };

    let talk = if input.is_dir() {
        info!("Processing folder-based talk from {}", input.display());
        let parser = FolderParser::new(&input);
        parser
            .parse(title_override, date_override)
            .context("parse folder talk")?
    } else {
        info!("Processing markdown file from {}", input.display());
        let content =
            fs::read_to_string(&input).with_context(|| format!("reading {}", input.display()))?;
        let parser = FlatFileParser::new(content);
        parser
            .parse(title_override, date_override)
            .context("parse markdown file")?
    };

    let toml = toml::to_string_pretty(&talk).context("to TOML")?;

    if let Some(out) = &output {
        write_talk(out, &toml).context("write talk")?;
    } else {
        eprintln!("{toml}");
    }

    Ok(())
}

fn write_talk(out: &Path, toml: &str) -> anyhow::Result<()> {
    let writer = File::create(out).with_context(|| format!("creating {}", out.display()))?;
    let mut writer = BufWriter::new(writer);
    writer.write_all(toml.as_bytes()).context("writing data")?;

    Ok(())
}

/// Parse a date string in YYYY-MM-DD format.
///
/// This function validates and parses date strings from CLI arguments or date files.
/// The input must be in ISO date format (YYYY-MM-DD).
///
/// # Arguments
///
/// * `date_str` - A date string in YYYY-MM-DD format
///
/// # Returns
///
/// Returns a validated `Date` object on success, or an error if the format is invalid
/// or the date is not valid (e.g., February 30th).
///
/// # Errors
///
/// This function will return an error if:
/// - The date string is not in YYYY-MM-DD format
/// - The date represents an invalid calendar date (e.g., 2023-02-29)
/// - Internal regex compilation fails (should not happen with valid regex)
///
/// # Examples
///
/// ```
/// # use toboggan_cli::*;
/// # fn example() -> anyhow::Result<()> {
/// // Valid dates
/// let date1 = parse_date_string("2024-12-31")?;
/// let date2 = parse_date_string("2024-02-29")?; // Leap year
///
/// // Invalid format
/// assert!(parse_date_string("2024/12/31").is_err());
/// assert!(parse_date_string("31-12-2024").is_err());
///
/// // Invalid date
/// assert!(parse_date_string("2023-02-29").is_err()); // Not a leap year
/// # Ok(())
/// # }
/// ```
pub fn parse_date_string(date_str: &str) -> anyhow::Result<Date> {
    let regex = regex::Regex::new(r"^(\d{4})-(\d{1,2})-(\d{1,2})$")?;

    if let Some(caps) = regex.captures(date_str) {
        let year = caps[1]
            .parse::<i16>()
            .with_context(|| format!("invalid year '{}'", &caps[1]))?;
        let month = caps[2]
            .parse::<i8>()
            .with_context(|| format!("invalid month '{}'", &caps[2]))?;
        let day = caps[3]
            .parse::<i8>()
            .with_context(|| format!("invalid day '{}'", &caps[3]))?;
        let date = Date::new(year, month, day).context("build date")?;

        Ok(date)
    } else {
        bail!("date must be in YYYY-MM-DD format, got '{}'", date_str)
    }
}
