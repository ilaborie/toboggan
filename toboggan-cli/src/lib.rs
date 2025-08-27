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
// pulldown-cmark completely replaced with comrak
use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, ComrakOptions, markdown_to_html, parse_document};
use toboggan_core::{Content, Date, Slide, SlideKind, Talk};
use tracing::{debug, info};

mod settings;
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
        parse_folder_talk(&input, title_override, date_override).context("parse folder talk")?
    } else {
        info!("Processing markdown file from {}", input.display());
        let content =
            fs::read_to_string(&input).with_context(|| format!("reading {}", input.display()))?;
        parse_content(&content, title_override, date_override)
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

fn scan_directory_structure(input_dir: &Path) -> anyhow::Result<Vec<std::fs::DirEntry>> {
    debug!("Scanning folder structure in {}", input_dir.display());

    let mut entries: Vec<_> = fs::read_dir(input_dir)
        .with_context(|| format!("reading directory {}", input_dir.display()))?
        .collect::<Result<Vec<_>, _>>()?;

    // Sort by filename for consistent ordering
    entries.sort_by_key(std::fs::DirEntry::file_name);
    Ok(entries)
}

fn extract_talk_metadata(
    input_dir: &Path,
    title_override: Option<Content>,
    date_override: Option<Date>,
) -> (Content, Date, Content) {
    // Use title override from CLI, fallback to title file, then folder name
    let title = title_override
        .or_else(|| find_title_in_folder(input_dir))
        .unwrap_or_else(|| {
            let folder_name = input_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Untitled Talk");
            Content::from(folder_name)
        });

    // Use date override from CLI, fallback to date file, then today
    let date = date_override
        .or_else(|| find_date_in_folder(input_dir))
        .unwrap_or_else(Date::today);

    // Look for footer in folder
    let footer = find_footer_in_folder(input_dir).unwrap_or_default();

    (title, date, footer)
}

fn should_skip_entry(filename_str: &str) -> bool {
    filename_str.starts_with('.')
        || filename_str == "title.md"
        || filename_str == "title.txt"
        || filename_str == "date.txt"
        || filename_str == "_cover.md"
        || filename_str == "_footer.md"
}

fn process_cover_slide(entries: &[std::fs::DirEntry], talk: &mut Talk) -> anyhow::Result<()> {
    for entry in entries {
        let path = entry.path();
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        if filename_str == "_cover.md" && path.is_file() {
            debug!("Processing cover slide: {}", path.display());
            let cover_slide = create_slide_from_file(&path)?;
            *talk = talk.clone().add_slide(cover_slide);
            break;
        }
    }
    Ok(())
}

fn process_directory_entries(
    entries: Vec<std::fs::DirEntry>,
    talk: &mut Talk,
) -> anyhow::Result<()> {
    for entry in entries {
        let path = entry.path();
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        if should_skip_entry(&filename_str) {
            continue;
        }

        if path.is_dir() {
            // Folder becomes a Part slide
            debug!("Processing folder as part: {}", path.display());
            let part_slide = create_part_slide_from_folder(&path)?;
            *talk = talk.clone().add_slide(part_slide);

            // Process contents of the folder
            let folder_slides = parse_folder_contents(&path)?;
            for slide in folder_slides {
                *talk = talk.clone().add_slide(slide);
            }
        } else if is_slide_file(&path) {
            // File becomes a slide (but not _cover.md)
            debug!("Processing file as slide: {}", path.display());
            let slide = create_slide_from_file(&path)?;
            *talk = talk.clone().add_slide(slide);
        }
    }
    Ok(())
}

/// Parse folder-based talk structure
fn parse_folder_talk(
    input_dir: &Path,
    title_override: Option<Content>,
    date_override: Option<Date>,
) -> anyhow::Result<Talk> {
    let entries = scan_directory_structure(input_dir)?;
    let (title, date, footer) = extract_talk_metadata(input_dir, title_override, date_override);

    let mut talk = Talk::new(title).with_date(date).with_footer(footer);

    process_cover_slide(&entries, &mut talk)?;
    process_directory_entries(entries, &mut talk)?;

    Ok(talk)
}

/// Find title in folder (title.md or title.txt)
fn find_title_in_folder(folder: &Path) -> Option<Content> {
    let title_md = folder.join("title.md");
    let title_txt = folder.join("title.txt");

    if title_md.exists()
        && let Ok(content) = fs::read_to_string(&title_md)
    {
        return Some(Content::from(content.trim()));
    }

    if title_txt.exists()
        && let Ok(content) = fs::read_to_string(&title_txt)
    {
        return Some(Content::from(content.trim()));
    }

    None
}

/// Find date in folder (date.txt file or today)
fn find_date_in_folder(folder: &Path) -> Option<Date> {
    let date_file = folder.join("date.txt");
    if date_file.exists()
        && let Ok(date_str) = fs::read_to_string(&date_file)
    {
        // Try to parse different date formats
        let date_str = date_str.trim();

        // Use the existing parse_date_string function
        if let Ok(date) = parse_date_string(date_str) {
            return Some(date);
        }
    }
    None
}

/// Find footer in folder (_footer.md file)
fn find_footer_in_folder(folder: &Path) -> Option<Content> {
    let footer_file = folder.join("_footer.md");

    if footer_file.exists()
        && let Ok(content) = fs::read_to_string(&footer_file)
    {
        let content = content.trim();
        if content.is_empty() {
            return None;
        }

        // Use comrak to convert markdown to HTML
        return Some(markdown_to_html_content(content));
    }

    None
}

/// Check if file should be processed as a slide
fn is_slide_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|extension| extension.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            "md" | "markdown" | "html" | "htm"
        )
    } else {
        false
    }
}

/// Create a Part slide from a folder
fn create_part_slide_from_folder(folder: &Path) -> anyhow::Result<Slide> {
    let part_md = folder.join("_part.md");

    if part_md.exists() {
        // Use _part.md file for part slide content
        let content = fs::read_to_string(&part_md)
            .with_context(|| format!("reading {}", part_md.display()))?;
        let slide = parse_markdown_to_slide(&content, SlideKind::Part);
        Ok(slide)
    } else {
        // Use folder name as part title
        let folder_name = folder
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled Part");
        Ok(Slide::part(folder_name))
    }
}

/// Process all slide files in a folder
fn parse_folder_contents(folder: &Path) -> anyhow::Result<Vec<Slide>> {
    let mut slides = Vec::new();

    let mut entries: Vec<_> = fs::read_dir(folder)
        .with_context(|| format!("reading directory {}", folder.display()))?
        .collect::<Result<Vec<_>, _>>()?;

    // Sort by filename for consistent ordering
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        // Skip special files and hidden files
        if filename_str.starts_with('.') || filename_str.starts_with('_') {
            continue;
        }

        if path.is_file() && is_slide_file(&path) {
            debug!("Processing folder content file: {}", path.display());
            let slide = create_slide_from_file(&path)?;
            slides.push(slide);
        }
    }

    Ok(slides)
}

/// Create a slide from a file
fn create_slide_from_file(file_path: &Path) -> anyhow::Result<Slide> {
    let filename = file_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("untitled");

    // Determine slide kind from filename
    let slide_kind = if filename == "_cover" {
        SlideKind::Cover
    } else if filename == "_part" {
        SlideKind::Part
    } else {
        SlideKind::Standard
    };

    let content = fs::read_to_string(file_path)
        .with_context(|| format!("reading {}", file_path.display()))?;

    let slide = if file_path.extension().and_then(|ext| ext.to_str()) == Some("html")
        || file_path.extension().and_then(|ext| ext.to_str()) == Some("htm")
    {
        // HTML file - use content directly
        create_html_slide(&content, slide_kind, filename)
    } else {
        // Markdown file - parse it
        parse_markdown_to_slide(&content, slide_kind)
    };

    Ok(slide)
}

/// Create slide from HTML content
fn create_html_slide(content: &str, slide_kind: SlideKind, filename: &str) -> Slide {
    let html_content = Content::html(content.trim());

    let slide = match slide_kind {
        SlideKind::Cover => Slide::cover(filename),
        SlideKind::Part => Slide::part(filename),
        SlideKind::Standard => Slide::new(filename),
    };

    slide.with_body(html_content)
}

/// Parse markdown content into a slide
fn parse_markdown_to_slide(content: &str, default_kind: SlideKind) -> Slide {
    // Use the comrak-based implementation
    parse_slide_from_markdown_comrak(content, default_kind)
}

fn parse_content(text: &str, title_override: Option<Content>, date_override: Option<Date>) -> Talk {
    // Use comrak-based flat file parsing
    parse_flat_markdown_comrak(text, title_override, date_override)
}

/// Parse flat markdown file using comrak
fn parse_flat_markdown_comrak(
    content: &str,
    title_override: Option<Content>,
    date_override: Option<Date>,
) -> Talk {
    let arena = Arena::new();
    let options = ComrakOptions::default();
    let root = parse_document(&arena, content, &options);

    let mut slides = Vec::new();
    let mut title_content = None;
    let mut current_slide_content = String::new();
    let mut found_title = false;
    let mut in_slide = false;

    // First pass: find title (first H1)
    for node in root.children() {
        if let NodeValue::Heading(ref heading) = node.data.borrow().value
            && heading.level == 1
            && !found_title
        {
            title_content = Some(extract_node_text(node));
            found_title = true;
            break;
        }
    }

    // Second pass: process slides
    for node in root.children() {
        match &node.data.borrow().value {
            NodeValue::Heading(heading) => {
                if heading.level == 1 && found_title {
                    // Skip the title heading
                    continue;
                }
                if heading.level >= 2 {
                    // Start new slide
                    if in_slide && !current_slide_content.trim().is_empty() {
                        let slide_kind = if heading.level == 2 {
                            SlideKind::Part
                        } else {
                            SlideKind::Standard
                        };
                        let slide =
                            parse_slide_from_markdown_comrak(&current_slide_content, slide_kind);
                        slides.push(slide);
                        current_slide_content.clear();
                    }
                    in_slide = true;
                }
                current_slide_content.push_str(&node_to_commonmark(node));
            }
            NodeValue::ThematicBreak => {
                // End current slide on horizontal rule
                if in_slide && !current_slide_content.trim().is_empty() {
                    let slide = parse_slide_from_markdown_comrak(
                        &current_slide_content,
                        SlideKind::Standard,
                    );
                    slides.push(slide);
                    current_slide_content.clear();
                    in_slide = false;
                }
            }
            _ => {
                if in_slide {
                    current_slide_content.push_str(&node_to_commonmark(node));
                }
            }
        }
    }

    // Handle last slide
    if in_slide && !current_slide_content.trim().is_empty() {
        let slide = parse_slide_from_markdown_comrak(&current_slide_content, SlideKind::Standard);
        slides.push(slide);
    }

    // Convert first slide to cover slide if it exists
    if let Some(first_slide) = slides.get_mut(0)
        && matches!(first_slide.kind, SlideKind::Standard)
    {
        *first_slide = Slide::cover(first_slide.title.clone())
            .with_body(first_slide.body.clone())
            .with_notes(first_slide.notes.clone());
    }

    let title = title_override
        .or_else(|| title_content.map(Content::text))
        .unwrap_or_else(|| Content::text("Untitled"));

    let mut talk = Talk::new(title);
    if let Some(date) = date_override {
        talk = talk.with_date(date);
    }

    for slide in slides {
        talk = talk.add_slide(slide);
    }

    talk
}

// ===== COMRAK CONVERSION FUNCTIONS =====

/// Extract text content from an AST node (recursive)
fn extract_node_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();

    match &node.data.borrow().value {
        NodeValue::Text(content) => {
            text.push_str(content);
        }
        NodeValue::Code(code) => {
            text.push_str(&code.literal);
        }
        NodeValue::SoftBreak => {
            text.push(' ');
        }
        NodeValue::LineBreak => {
            text.push('\n');
        }
        _ => {
            // Recursively extract text from children
            for child in node.children() {
                text.push_str(&extract_node_text(child));
            }
        }
    }

    text
}

/// Convert markdown to HTML content using comrak
fn markdown_to_html_content(markdown: &str) -> Content {
    if markdown.trim().is_empty() {
        return Content::Empty;
    }

    let options = ComrakOptions::default();
    let html = markdown_to_html(markdown, &options);

    Content::Html {
        raw: html.trim().to_string(),
        alt: if markdown.is_empty() {
            None
        } else {
            Some(markdown.to_string())
        },
    }
}

/// Parse slide content from markdown using comrak
fn parse_slide_from_markdown_comrak(content: &str, default_kind: SlideKind) -> Slide {
    let arena = Arena::new();
    let options = ComrakOptions::default();
    let root = parse_document(&arena, content, &options);

    let mut title = None;
    let mut body_content = String::new();
    let mut notes_content = String::new();
    let mut in_notes = false;
    let mut found_title = false;

    for node in root.children() {
        match &node.data.borrow().value {
            NodeValue::Heading(heading) => {
                if !found_title && heading.level <= 3 {
                    title = Some(extract_node_text(node));
                    found_title = true;
                } else if heading.level >= 4 {
                    // Check if this is a notes heading
                    let heading_text = extract_node_text(node);
                    if heading_text.to_lowercase().contains("note") {
                        in_notes = true;
                        continue;
                    }
                }

                if !in_notes {
                    body_content.push_str(&node_to_commonmark(node));
                }
            }
            NodeValue::BlockQuote => {
                // Treat blockquotes as notes (legacy support)
                notes_content.push_str(&extract_blockquote_text(node));
            }
            _ => {
                if in_notes {
                    notes_content.push_str(&node_to_commonmark(node));
                } else {
                    body_content.push_str(&node_to_commonmark(node));
                }
            }
        }
    }

    let slide_title = title.map_or(Content::text(""), Content::text);
    let slide_body = if body_content.trim().is_empty() {
        Content::Empty
    } else {
        markdown_to_html_content(&body_content)
    };
    let slide_notes = if notes_content.trim().is_empty() {
        None
    } else {
        Some(Content::text(notes_content.trim()))
    };

    let slide = match default_kind {
        SlideKind::Cover => Slide::cover(slide_title),
        SlideKind::Part => Slide::part(slide_title),
        SlideKind::Standard => Slide::new(slide_title),
    };

    let slide = slide.with_body(slide_body);
    if let Some(notes) = slide_notes {
        slide.with_notes(notes)
    } else {
        slide
    }
}

/// Convert AST node to `CommonMark` markdown (simplified version)
fn node_to_commonmark<'a>(node: &'a AstNode<'a>) -> String {
    // For now, just extract the text and add basic markdown formatting
    // This is a simplified version - full node conversion is complex
    match &node.data.borrow().value {
        NodeValue::Heading(heading) => {
            let prefix = "#".repeat(heading.level as usize);
            let text = extract_node_text(node);
            format!("{} {}\n\n", prefix, text.trim())
        }
        NodeValue::Paragraph => {
            let text = extract_node_text(node);
            format!("{}\n\n", text.trim())
        }
        NodeValue::BlockQuote => {
            let text = extract_node_text(node);
            text.lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n\n"
        }
        _ => {
            let text = extract_node_text(node);
            if text.trim().is_empty() {
                String::new()
            } else {
                format!("{}\n\n", text.trim())
            }
        }
    }
}

/// Extract text from blockquote nodes
fn extract_blockquote_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();

    if let NodeValue::BlockQuote = &node.data.borrow().value {
        for child in node.children() {
            text.push_str(&extract_node_text(child));
            text.push('\n');
        }
    }

    text.trim().to_string()
}
