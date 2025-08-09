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
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
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
        parse_content(&content, title_override, date_override).context("parse content")?
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

/// Parse folder-based talk structure
fn parse_folder_talk(
    input_dir: &Path,
    title_override: Option<Content>,
    date_override: Option<Date>,
) -> anyhow::Result<Talk> {
    debug!("Scanning folder structure in {}", input_dir.display());

    // Read all entries and sort them
    let mut entries: Vec<_> = fs::read_dir(input_dir)
        .with_context(|| format!("reading directory {}", input_dir.display()))?
        .collect::<Result<Vec<_>, _>>()?;

    // Sort by filename for consistent ordering
    entries.sort_by_key(std::fs::DirEntry::file_name);

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

    let mut talk = Talk::new(title).with_date(date).with_footer(footer);

    // First pass: look for cover slide
    for entry in &entries {
        let path = entry.path();
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        if filename_str == "_cover.md" && path.is_file() {
            debug!("Processing cover slide: {}", path.display());
            let cover_slide = create_slide_from_file(&path)?;
            talk = talk.add_slide(cover_slide);
            break;
        }
    }

    // Second pass: process other entries in order
    for entry in entries {
        let path = entry.path();
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        // Skip hidden files, special files, and cover slide (already processed)
        if filename_str.starts_with('.')
            || filename_str == "title.md"
            || filename_str == "title.txt"
            || filename_str == "date.txt"
            || filename_str == "_cover.md"
            || filename_str == "_footer.md"
        {
            continue;
        }

        if path.is_dir() {
            // Folder becomes a Part slide
            debug!("Processing folder as part: {}", path.display());
            let part_slide = create_part_slide_from_folder(&path)?;
            talk = talk.add_slide(part_slide);

            // Process contents of the folder
            let folder_slides = parse_folder_contents(&path)?;
            for slide in folder_slides {
                talk = talk.add_slide(slide);
            }
        } else if is_slide_file(&path) {
            // File becomes a slide (but not _cover.md)
            debug!("Processing file as slide: {}", path.display());
            let slide = create_slide_from_file(&path)?;
            talk = talk.add_slide(slide);
        }
    }

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

        // Try YYYY-MM-DD format
        if let Some(caps) = regex::Regex::new(r"^(\d{4})-(\d{1,2})-(\d{1,2})$")
            .ok()?
            .captures(date_str)
            && let (Ok(year), Ok(month), Ok(day)) = (
                caps[1].parse::<i16>(),
                caps[2].parse::<i8>(),
                caps[3].parse::<i8>(),
            )
        {
            return Date::new(year, month, day).ok();
        }
    }
    None
}

/// Find footer in folder (_footer.md file)
fn find_footer_in_folder(folder: &Path) -> Option<Content> {
    use pulldown_cmark::{Options, Parser, html};

    let footer_file = folder.join("_footer.md");

    if footer_file.exists()
        && let Ok(content) = fs::read_to_string(&footer_file)
    {
        let content = content.trim();
        if content.is_empty() {
            return None;
        }

        // Parse markdown content to HTML
        let options = Options::all();
        let parser = Parser::new_ext(content, options);

        let mut html = String::new();
        html::push_html(&mut html, parser);

        return Some(Content::Html {
            raw: html.trim().to_string(),
            alt: Some(content.to_string()),
        });
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
    let mut state = SlideParseState::new(default_kind);
    let options = Options::all();

    let parser = Parser::new_ext(content, options);
    for event in parser {
        state.consume(&event);
    }

    state.finish()
}

fn parse_content(
    text: &str,
    title_override: Option<Content>,
    date_override: Option<Date>,
) -> anyhow::Result<Talk> {
    let mut state = TalkParseState::default();
    let options = Options::all();

    let parser = Parser::new_ext(text, options);
    for event in parser {
        state
            .consume(&event)
            .with_context(|| format!("processing {event:?}"))?;
    }

    let mut talk = state.finish().context("finish parsing")?;

    // Apply title override if provided
    if let Some(title) = title_override {
        talk.title = title;
    }

    // Apply date override if provided
    if let Some(date) = date_override {
        talk.date = date;
    }

    Ok(talk)
}

/// State machine for parsing individual slide markdown
#[derive(Debug, Clone)]
struct SlideParseState<'i> {
    slide_kind: SlideKind,
    title_events: Vec<Event<'i>>,
    body_events: Vec<Event<'i>>,
    notes_events: Vec<Event<'i>>,
    in_title: bool,
    in_notes: bool,
}

impl<'i> SlideParseState<'i> {
    fn new(slide_kind: SlideKind) -> Self {
        Self {
            slide_kind,
            title_events: Vec::new(),
            body_events: Vec::new(),
            notes_events: Vec::new(),
            in_title: false,
            in_notes: false,
        }
    }

    fn consume(&mut self, event: &Event<'i>) {
        match event {
            Event::Start(Tag::Heading { level, classes, .. }) => match level {
                HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3 => {
                    self.in_title = true;
                    return;
                }
                HeadingLevel::H4 | HeadingLevel::H5 | HeadingLevel::H6 => {
                    if classes.iter().any(|class| class.as_ref() == "notes") {
                        self.in_notes = true;
                        return;
                    }
                }
            },
            Event::End(TagEnd::Heading(level)) => {
                if matches!(
                    level,
                    HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3
                ) {
                    self.in_title = false;
                    return;
                } else if matches!(
                    level,
                    HeadingLevel::H4 | HeadingLevel::H5 | HeadingLevel::H6
                ) && self.in_notes
                {
                    self.in_notes = false;
                    return;
                }
            }
            // Fallback to blockquote for notes (for compatibility)
            Event::Start(Tag::BlockQuote(_)) => {
                self.in_notes = true;
                return;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                self.in_notes = false;
                return;
            }
            _ => {}
        }

        if self.in_title {
            self.title_events.push(event.clone());
        } else if self.in_notes {
            self.notes_events.push(event.clone());
        } else {
            self.body_events.push(event.clone());
        }
    }

    fn finish(self) -> Slide {
        let title = if self.title_events.is_empty() {
            Content::Empty
        } else {
            events_to_content(&self.title_events)
        };

        let body = if self.body_events.is_empty() {
            Content::Empty
        } else {
            events_to_markdown_content(&self.body_events)
        };

        let notes = if self.notes_events.is_empty() {
            Content::Empty
        } else {
            events_to_content(&self.notes_events)
        };

        let slide = match self.slide_kind {
            SlideKind::Cover => Slide::cover(title),
            SlideKind::Part => Slide::part(title),
            SlideKind::Standard => Slide::new(title),
        };

        slide.with_body(body).with_notes(notes)
    }
}

#[derive(Debug, Clone, Default)]
enum TalkParseState<'i> {
    #[default]
    Init,

    Title {
        current: Vec<Event<'i>>,
    },

    Slide {
        talk: Talk,
        current: Vec<Event<'i>>,
        is_first_slide: bool,
    },
}

impl<'i> TalkParseState<'i> {
    fn consume(&mut self, event: &Event<'i>) -> anyhow::Result<()> {
        match self {
            Self::Init => {
                if let Event::Start(Tag::Heading { level, .. }) = event
                    && level == &HeadingLevel::H1
                {
                    *self = Self::Title { current: vec![] };
                } else {
                    bail!("expected a heading level 1, got {event:?}");
                }
            }
            Self::Title { current } => {
                if let Event::End(TagEnd::Heading(level)) = event
                    && level == &HeadingLevel::H1
                {
                    let title = events_to_content(current);
                    let talk = Talk::new(title);
                    *self = Self::Slide {
                        talk,
                        current: vec![],
                        is_first_slide: true,
                    };
                } else {
                    current.push(event.clone());
                }
            }
            Self::Slide {
                talk,
                current,
                is_first_slide,
            } => {
                if let Event::Rule = event {
                    if !current.is_empty() {
                        let slide = events_to_slide(current);
                        let slide = if *is_first_slide {
                            // Convert to cover slide by extracting components
                            let Slide {
                                title, body, notes, ..
                            } = slide;
                            Slide::cover(title).with_body(body).with_notes(notes)
                        } else {
                            slide
                        };
                        if *is_first_slide {
                            *is_first_slide = false;
                        }
                        talk.slides.push(slide);
                        current.clear();
                    }
                } else {
                    current.push(event.clone());
                }
            }
        }

        Ok(())
    }

    fn finish(self) -> anyhow::Result<Talk> {
        match self {
            Self::Slide {
                mut talk,
                current,
                is_first_slide,
            } => {
                if !current.is_empty() {
                    let slide = events_to_slide(&current);
                    let slide = if is_first_slide {
                        // Convert to cover slide by extracting components
                        let Slide {
                            title, body, notes, ..
                        } = slide;
                        Slide::cover(title).with_body(body).with_notes(notes)
                    } else {
                        slide
                    };
                    talk.slides.push(slide);
                }
                Ok(talk)
            }
            _ => bail!("invalid state: expected to be in Slide state at finish"),
        }
    }
}

fn events_to_content(events: &[Event]) -> Content {
    let mut text = String::new();

    for event in events {
        match event {
            Event::Text(text_content) => text.push_str(text_content),
            Event::Code(code_content) => {
                text.push('`');
                text.push_str(code_content);
                text.push('`');
            }
            Event::SoftBreak => text.push(' '),
            Event::HardBreak => text.push('\n'),
            _ => {}
        }
    }

    Content::Text {
        text: text.trim().to_string(),
    }
}

fn events_to_slide(events: &[Event]) -> Slide {
    let mut title_events = Vec::new();
    let mut body_events = Vec::new();
    let mut notes_events = Vec::new();
    let mut in_title = false;
    let mut in_notes = false;
    let mut slide_kind = SlideKind::Standard;

    for event in events {
        match event {
            Event::Start(Tag::Heading { level, classes, .. }) => match level {
                HeadingLevel::H2 => {
                    in_title = true;
                    slide_kind = SlideKind::Part;
                    continue;
                }
                HeadingLevel::H3 => {
                    in_title = true;
                    continue;
                }
                HeadingLevel::H4 | HeadingLevel::H5 | HeadingLevel::H6 => {
                    if classes.iter().any(|class| class.as_ref() == "notes") {
                        in_notes = true;
                        continue;
                    }
                }
                HeadingLevel::H1 => {}
            },
            Event::End(TagEnd::Heading(level)) => {
                if matches!(level, HeadingLevel::H2 | HeadingLevel::H3) {
                    in_title = false;
                    continue;
                } else if matches!(
                    level,
                    HeadingLevel::H4 | HeadingLevel::H5 | HeadingLevel::H6
                ) && in_notes
                {
                    in_notes = false;
                    continue;
                }
            }
            // Fallback to blockquote for notes (for compatibility)
            Event::Start(Tag::BlockQuote(_)) => {
                in_notes = true;
                continue;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                in_notes = false;
                continue;
            }
            _ => {}
        }

        if in_title {
            title_events.push(event.clone());
        } else if in_notes {
            notes_events.push(event.clone());
        } else {
            body_events.push(event.clone());
        }
    }

    let title = if title_events.is_empty() {
        Content::Empty
    } else {
        events_to_content(&title_events)
    };

    let body = if body_events.is_empty() {
        Content::Empty
    } else {
        events_to_markdown_content(&body_events)
    };

    let notes = if notes_events.is_empty() {
        Content::Empty
    } else {
        events_to_content(&notes_events)
    };

    let slide = match slide_kind {
        SlideKind::Cover => Slide::cover(title),
        SlideKind::Part => Slide::part(title),
        SlideKind::Standard => Slide::new(title),
    };

    slide.with_body(body).with_notes(notes)
}

fn events_to_markdown_content(events: &[Event]) -> Content {
    use pulldown_cmark::html;

    let mut html = String::new();
    html::push_html(&mut html, events.iter().cloned());

    let alt = events_to_markdown_text(events);

    if html.trim().is_empty() {
        Content::Empty
    } else {
        Content::Html {
            raw: html.trim().to_string(),
            alt: if alt.is_empty() { None } else { Some(alt) },
        }
    }
}

fn events_to_markdown_text(events: &[Event]) -> String {
    let mut markdown = String::new();
    let mut list_depth: usize = 0;
    let mut in_code_block = false;

    for event in events {
        match event {
            Event::Start(Tag::Paragraph) => {
                if !markdown.is_empty() && !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
            }
            Event::End(TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item) => {
                markdown.push('\n');
            }
            Event::Start(Tag::Heading { level, .. }) => {
                if !markdown.is_empty() && !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
                match level {
                    pulldown_cmark::HeadingLevel::H1 => markdown.push('#'),
                    pulldown_cmark::HeadingLevel::H2 => markdown.push_str("##"),
                    pulldown_cmark::HeadingLevel::H3 => markdown.push_str("###"),
                    pulldown_cmark::HeadingLevel::H4 => markdown.push_str("####"),
                    pulldown_cmark::HeadingLevel::H5 => markdown.push_str("#####"),
                    pulldown_cmark::HeadingLevel::H6 => markdown.push_str("######"),
                }
                markdown.push(' ');
            }
            Event::Start(Tag::List(_)) => {
                list_depth += 1;
                if !markdown.is_empty() && !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => {
                for _ in 0..(list_depth - 1) {
                    markdown.push_str("  ");
                }
                markdown.push_str("- ");
            }
            Event::Start(Tag::CodeBlock(_)) => {
                if !markdown.is_empty() && !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
                markdown.push_str("```\n");
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                if !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
                markdown.push_str("```\n");
                in_code_block = false;
            }
            Event::Text(text) => {
                markdown.push_str(text);
            }
            Event::Code(code) => {
                if in_code_block {
                    markdown.push_str(code);
                } else {
                    markdown.push('`');
                    markdown.push_str(code);
                    markdown.push('`');
                }
            }
            Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) => markdown.push_str("**"),
            Event::Start(Tag::Emphasis) | Event::End(TagEnd::Emphasis) => markdown.push('*'),
            Event::SoftBreak => markdown.push(' '),
            Event::HardBreak => markdown.push('\n'),
            Event::Rule => {
                if !markdown.is_empty() && !markdown.ends_with('\n') {
                    markdown.push('\n');
                }
                markdown.push_str("---\n");
            }
            _ => {}
        }
    }

    markdown.trim().to_string()
}
