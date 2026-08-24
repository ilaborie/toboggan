//! Parses a folder of Markdown into a [`toboggan_core::Talk`], and renders a
//! `Talk` back out as TOML, JSON, YAML, HTML, Typst, or a folder of thumbnails.
//!
//! A deck is a directory: `_cover.md` for the cover, `_part.md` for a section
//! title, `_head.html` and `_footer.html` for the chrome, `_preamble.typ` to
//! replace the generated Typst preamble, and numbered files and
//! folders for everything else. Ordering comes from the filenames.
//!
//! Errors are [`miette`] diagnostics, so a bad slide is reported with the file
//! and the span rather than a message about a string.
//!
//! See the crate README for the front matter keys and the body directives.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use toboggan_core::{Date, Slide, Talk};
use tracing::debug;

pub mod error;
pub use self::error::{Result, TobogganCliError};

pub mod parser;
use parser::{FolderParser, Overrides};

pub mod mermaid;

pub mod output;

pub mod scaffold;

mod settings;
pub use self::settings::*;

pub mod display;

pub mod stats;

/// The one directory a deck's assets live in, beside its slides folder.
const PUBLIC_DIR: &str = "public";

/// Not `Clone`: a failure carries its whole [`TobogganCliError`], which owns
/// non-cloneable sources such as `io::Error`. Keeping the diagnostic rather
/// than its rendered text is what lets [`TobogganCliError::SlidesFailedToParse`]
/// draw each failure's own snippet.
#[derive(Debug)]
pub enum SlideProcessingResult {
    Processed(Slide),
    Skipped(Slide),
    Ignored(String),
    Error(Box<TobogganCliError>),
}

#[derive(Debug, Clone)]
pub struct TalkMetadata {
    pub title: String,
    pub date: Date,
    pub footer: Option<String>,
    pub head: Option<String>,
    /// Typst preamble replacing the generated one; see
    /// [`toboggan_core::Talk::typst_preamble`].
    pub typst_preamble: Option<String>,
    /// BCP 47 language tag for the deck; see [`toboggan_core::Talk::lang`].
    pub lang: Option<String>,
    /// Default working directory for the `QuakeTerminal` overlay (talk-level fallback).
    pub default_terminal_cwd: Option<String>,
    /// Source directory of the talk; used to resolve relative quake cwds.
    pub source_dir: Option<PathBuf>,
}

impl Default for TalkMetadata {
    fn default() -> Self {
        Self {
            title: "Unknown Talk".to_owned(),
            date: Date::today(),
            footer: None,
            head: None,
            typst_preamble: None,
            lang: None,
            default_terminal_cwd: None,
            source_dir: None,
        }
    }
}

#[derive(Debug)]
pub struct ParseResult {
    pub talk_metadata: TalkMetadata,
    pub slides: Vec<SlideProcessingResult>,
}

impl ParseResult {
    #[must_use]
    pub fn to_talk(&self) -> Talk {
        let mut talk = Talk::new(&self.talk_metadata.title);
        talk.date = self.talk_metadata.date;
        talk.footer.clone_from(&self.talk_metadata.footer);
        talk.head.clone_from(&self.talk_metadata.head);
        talk.typst_preamble
            .clone_from(&self.talk_metadata.typst_preamble);
        talk.lang.clone_from(&self.talk_metadata.lang);
        talk.default_terminal_cwd
            .clone_from(&self.talk_metadata.default_terminal_cwd);
        talk.source_dir = self
            .talk_metadata
            .source_dir
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());

        for slide_result in &self.slides {
            if let SlideProcessingResult::Processed(slide) = slide_result {
                talk.slides.push(slide.clone());
            }
        }

        // Bake resolved (absolute) quake cwds into each slide so consumers (server,
        // wasm frontend) don't need access to talk.source_dir, which is not serialized.
        let resolved = talk
            .slides
            .iter()
            .map(|slide| slide.resolved_quake_cwd(&talk))
            .collect::<Vec<_>>();
        for (slide, cwd) in talk.slides.iter_mut().zip(resolved) {
            slide.quake_terminal_cwd = cwd;
        }

        talk
    }

    /// The diagnostic for every slide that failed to parse, in discovery order.
    ///
    /// [`Self::to_talk`] silently drops those slides, so any caller that renders
    /// or analyses the talk has to decide what to do about them. Without this a
    /// single front-matter typo makes a slide vanish from the deck while the
    /// command still reports success.
    #[must_use]
    pub fn errors(&self) -> Vec<&TobogganCliError> {
        self.slides
            .iter()
            .filter_map(|slide_result| match slide_result {
                SlideProcessingResult::Error(error) => Some(&**error),
                _ => None,
            })
            .collect()
    }

    /// Moves the failures out, so they can be attached to a diagnostic.
    ///
    /// `#[related]` needs owned values, and the slides they came from are
    /// dropped by [`Self::to_talk`] anyway.
    ///
    /// The failed slides are *removed*, so afterwards [`Self::errors`] is empty
    /// and [`Self::stats`] counts a deck that looks whole. Call it last, after
    /// anything that reports numbers.
    #[must_use]
    pub fn take_errors(&mut self) -> Vec<TobogganCliError> {
        let mut failures = Vec::new();
        self.slides = std::mem::take(&mut self.slides)
            .into_iter()
            .filter_map(|slide_result| match slide_result {
                SlideProcessingResult::Error(error) => {
                    failures.push(*error);
                    None
                }
                kept => Some(kept),
            })
            .collect();
        failures
    }

    #[must_use]
    pub fn stats(&self) -> ParseStats {
        let mut stats = ParseStats::default();

        for slide_result in &self.slides {
            match slide_result {
                SlideProcessingResult::Processed(_) => stats.processed += 1,
                SlideProcessingResult::Skipped(_) => stats.skipped += 1,
                SlideProcessingResult::Ignored(_) => stats.ignored += 1,
                SlideProcessingResult::Error(_) => stats.errors += 1,
            }
        }

        stats
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParseStats {
    pub processed: usize,
    pub skipped: usize,
    pub ignored: usize,
    pub errors: usize,
}

impl ParseStats {
    #[must_use]
    pub fn total(&self) -> usize {
        self.processed + self.skipped + self.ignored + self.errors
    }
}

pub fn add_counters_to_slides(parse_result: &mut ParseResult) {
    let mut part_number = 0;
    let mut slide_in_part = 0;
    let mut in_part = false;

    for slide_result in &mut parse_result.slides {
        match slide_result {
            SlideProcessingResult::Processed(slide) => {
                if slide.kind == toboggan_core::SlideKind::Part {
                    part_number += 1;
                    slide_in_part = 0;
                    in_part = true;
                } else if in_part {
                    slide_in_part += 1;
                }
            }
            SlideProcessingResult::Skipped(slide)
                if slide.kind == toboggan_core::SlideKind::Part =>
            {
                in_part = false;
            }
            _ => {}
        }

        match slide_result {
            SlideProcessingResult::Processed(slide) => {
                let counter = match slide.kind {
                    toboggan_core::SlideKind::Part => format!("{part_number}. "),
                    _ if in_part => format!("{part_number}.{slide_in_part} "),
                    _ => String::new(),
                };
                if !counter.is_empty() {
                    slide.title = format!("{counter}{}", slide.title).into();
                }
            }
            SlideProcessingResult::Skipped(_slide) => {}
            _ => {}
        }
    }
}

#[doc(hidden)]
#[allow(clippy::print_stdout)]
pub fn run(settings: &Settings) -> Result<()> {
    if settings.list_themes {
        list_available_themes();
        return Ok(());
    }

    // Before anything is parsed: the highlighter panics on a theme it cannot
    // find, from inside comrak, which is neither actionable nor catchable.
    if !parser::config::is_known_theme(&settings.theme) {
        return Err(TobogganCliError::UnknownTheme {
            theme: settings.theme.clone(),
        });
    }

    let input = validate_input(settings.input.as_ref())?;
    let mut parse_result = parse_presentation(input, settings)?;
    display_results(&parse_result, settings)?;

    // Fail before writing anything. `to_talk` drops unparseable slides, so
    // continuing here wrote a silently-truncated deck and still exited 0 — which
    // the GitHub Action turns into a published deck with slides missing.
    let failures = parse_result.take_errors();
    if !failures.is_empty() {
        return Err(TobogganCliError::SlidesFailedToParse { failures });
    }

    if let Some(output) = &settings.output {
        write_output(&parse_result, output, settings)?;
    } else {
        display::suggest_output_file(&mut std::io::stdout())
            .map_err(TobogganCliError::write_stdout)?;
    }

    Ok(())
}

fn validate_input(input: Option<&PathBuf>) -> Result<&PathBuf> {
    let input = input.ok_or_else(|| TobogganCliError::NotADirectory {
        path: PathBuf::from("no input provided"),
    })?;

    if !input.is_dir() {
        return Err(TobogganCliError::NotADirectory {
            path: input.clone(),
        });
    }

    Ok(input)
}

/// Parses a presentation folder into a [`ParseResult`], applying slide numbering
/// unless `settings.no_counter` is set.
///
/// Exposed so the unified CLI's build+serve and folder-watch paths can rebuild
/// the talk in-memory via [`ParseResult::to_talk`].
///
/// # Errors
/// Returns an error if the folder cannot be parsed.
/// Builds the deck's Mermaid renderer from `settings`.
///
/// Deliberately cheap to call more than once per build: it reads one small
/// JSON file, and having every path derive it from the same `Settings` is what
/// keeps the PDF and the web client drawing the same diagram.
///
/// # Errors
/// Returns an error if the configured file is missing or is not valid Mermaid
/// configuration JSON.
pub fn mermaid_renderer(settings: &Settings) -> Result<mermaid::MermaidRenderer> {
    mermaid::MermaidRenderer::from_config(settings.mermaid_config.as_deref())
}

pub fn parse_presentation(input: &Path, settings: &Settings) -> Result<ParseResult> {
    debug!("Processing folder-based talk from {}", input.display());

    let parser = FolderParser::new(
        input.to_path_buf(),
        settings.theme.clone(),
        mermaid_renderer(settings)?,
    )?;
    let mut parse_result = parser.parse(Overrides {
        title: settings.title.clone(),
        date: settings.date,
        lang: settings.lang.clone(),
    })?;

    // The flag wins over the deck's own `_preamble.typ`: one is what the deck
    // ships with, the other is what this run asked for.
    if let Some(path) = &settings.typst_preamble {
        let content = std::fs::read_to_string(path)
            .map_err(|err| TobogganCliError::read_file(path.clone(), err))?;
        // Blank means absent, as it does for the deck's own `_preamble.typ`.
        // Note this still overrides the deck file: pointing the flag at an empty
        // file asks for the generated preamble, not for the deck's.
        parse_result.talk_metadata.typst_preamble =
            Some(content).filter(|content| !content.trim().is_empty());
    }

    if !settings.no_counter {
        add_counters_to_slides(&mut parse_result);
    }

    Ok(parse_result)
}

fn display_results(parse_result: &ParseResult, settings: &Settings) -> Result<()> {
    let display_formatter = display::DisplayFormatter::new();
    display_formatter
        .display_results(parse_result, &mut std::io::stdout())
        .map_err(TobogganCliError::write_stdout)?;

    if !settings.no_stats {
        let stats = stats::PresentationStats::from_parse_result(
            parse_result,
            settings.wpm,
            !settings.exclude_notes_from_duration,
        );
        stats
            .display(
                &mut std::io::stdout(),
                display::DisplayConfig::should_use_colors(),
            )
            .map_err(TobogganCliError::write_stdout)?;
    }

    Ok(())
}

#[allow(clippy::print_stderr)]
fn write_output(parse_result: &ParseResult, output: &Path, settings: &Settings) -> Result<()> {
    let format = settings.resolve_format();
    let talk = parse_result.to_talk();
    let serialized = output::serialize_talk(
        &talk,
        format,
        settings.base_url.as_deref().unwrap_or_default(),
        &mermaid_renderer(settings)?,
    )?;

    write_talk(output, &serialized)?;

    // Count slides excluding Part slides (section dividers)
    let content_slide_count = talk
        .slides
        .iter()
        .filter(|slide| slide.kind != toboggan_core::SlideKind::Part)
        .count();
    if content_slide_count > 0 {
        eprintln!(
            "\n✅ Successfully wrote {} slides to {}",
            content_slide_count,
            output.display()
        );
    } else {
        eprintln!("\n⚠️  No slides were processed successfully. File not written.");
    }

    if matches!(format, OutputFormat::Html) {
        copy_public_assets(&talk, output)?;
    }

    Ok(())
}

/// Copies the deck's `public/` next to an exported HTML file.
///
/// The export references its assets relative to itself (see
/// [`output::serialize_talk`]), which is only true once they are actually
/// there. Without this the file is published with every image 404ing — which is
/// what the GitHub Action has been doing.
///
/// Nothing to do for a deck with no `public/`, and nothing to do when the
/// directory is already where it needs to be, which is the case for an export
/// written into the deck root.
#[allow(clippy::print_stderr)]
fn copy_public_assets(talk: &Talk, output: &Path) -> Result<()> {
    let Some(source) = output::deck_root(talk).map(|root| root.join(PUBLIC_DIR)) else {
        return Ok(());
    };
    if !source.is_dir() {
        return Ok(());
    }

    let destination = output
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(PUBLIC_DIR);
    if same_directory(&source, &destination) || is_inside(&source, &destination) {
        return Ok(());
    }

    let count = copy_dir(&source, &destination)?;
    eprintln!(
        "📦 Copied {count} asset(s) to {} so the deck's images resolve",
        destination.display()
    );
    Ok(())
}

/// Whether two paths name the same directory, as far as the filesystem knows.
///
/// Compared after canonicalising, so `deck/public` and `./deck/../deck/public`
/// are recognised as one and the copy is skipped rather than attempting to copy
/// a directory onto itself.
fn same_directory(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        // A destination that does not exist yet is the ordinary case, and it is
        // not the source. Anything else — a permission error, a symlink loop —
        // is not evidence that the two differ, but the copy below reports it
        // with the path it failed on, which is the better message.
        _ => false,
    }
}

/// Whether `destination` sits inside `source`.
///
/// Copying a directory into its own subtree never terminates: the destination
/// is created, then found in the source's listing, then descended into. The
/// deck that triggers it is ordinary — `public/` beside `slides/`, exported
/// with `-o public/index.html`.
fn is_inside(source: &Path, destination: &Path) -> bool {
    // The destination usually does not exist yet, so its nearest existing
    // ancestor is what can be canonicalised and compared.
    let Ok(source) = source.canonicalize() else {
        return false;
    };
    let mut candidate = destination.to_path_buf();
    loop {
        if let Ok(candidate) = candidate.canonicalize() {
            return candidate.starts_with(&source);
        }
        if !candidate.pop() {
            return false;
        }
    }
}

/// Recursively copies `source` into `destination`, returning the file count.
///
/// The listing is taken *before* the destination is created. Creating it first
/// meant that a destination inside the source — `-o public/index.html` in a deck
/// that has a `public/` — appeared in its own listing and was copied into
/// itself, without bound: `public/public/public/…` until the path length or the
/// disk gave out, shredding the author's assets on the way.
fn copy_dir(source: &Path, destination: &Path) -> Result<usize> {
    let entries = std::fs::read_dir(source)
        .map_err(|err| TobogganCliError::read_file(source.to_path_buf(), err))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|err| TobogganCliError::read_file(source.to_path_buf(), err))?;

    std::fs::create_dir_all(destination)
        .map_err(|err| TobogganCliError::create_file(destination.to_path_buf(), err))?;

    let mut count = 0;
    for entry in entries {
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|err| TobogganCliError::read_file(from.clone(), err))?;
        if file_type.is_symlink() {
            // A symlink to an ancestor recurses by the same mechanism, and a
            // deck's assets have no reason to contain one.
            continue;
        }
        if file_type.is_dir() {
            count += copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(|err| TobogganCliError::create_file(to, err))?;
            count += 1;
        }
    }
    Ok(count)
}

fn write_talk(out: &Path, content: &[u8]) -> Result<()> {
    let writer = File::create(out)
        .map_err(|source| TobogganCliError::create_file(out.to_path_buf(), source))?;
    let mut writer = BufWriter::new(writer);
    writer
        .write_all(content)
        .map_err(|source| TobogganCliError::write_file(out.to_path_buf(), source))?;

    Ok(())
}

/// Prints the themes the highlighter can load, from the one list that decides.
///
/// Generated rather than read from a text file: the file said twenty-two, the
/// highlighter knew seven, and the fifteen it did not know panicked.
#[allow(clippy::print_stdout)]
fn list_available_themes() {
    use crate::parser::config::{AVAILABLE_THEMES, DEFAULT_THEME};

    println!("Available syntax highlighting themes:\n");
    for theme in AVAILABLE_THEMES {
        let default = if theme == DEFAULT_THEME {
            " (default)"
        } else {
            ""
        };
        println!("  {theme}{default}");
    }
    println!("\nNote: theme names are case-sensitive.");
}

fn parse_date_string(date_str: &str) -> Result<Date> {
    date_str
        .parse::<Date>()
        .map_err(|_| TobogganCliError::InvalidDateFormat {
            input: date_str.to_owned(),
        })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_core::Slide;

    use super::*;

    fn create_test_parse_result_for_counter() -> ParseResult {
        let talk_metadata = TalkMetadata {
            title: "Test Presentation".to_owned(),
            date: Date::today(),
            footer: None,
            head: None,
            typst_preamble: None,
            lang: None,
            default_terminal_cwd: None,
            source_dir: None,
        };

        let slides = vec![
            SlideProcessingResult::Processed(Slide::new("Introduction")),
            SlideProcessingResult::Processed(Slide::part("Part One")),
            SlideProcessingResult::Processed(Slide::new("Topic A")),
            SlideProcessingResult::Skipped(Slide::new("Optional Topic")),
            SlideProcessingResult::Processed(Slide::new("Topic B")),
            SlideProcessingResult::Processed(Slide::part("Part Two")),
            SlideProcessingResult::Processed(Slide::new("Topic C")),
        ];

        ParseResult {
            talk_metadata,
            slides,
        }
    }

    #[test]
    #[allow(clippy::indexing_slicing)]
    fn test_add_counters_to_slides() {
        let mut parse_result = create_test_parse_result_for_counter();
        add_counters_to_slides(&mut parse_result);

        // Check the titles have counters added
        if let SlideProcessingResult::Processed(slide) = &parse_result.slides[0] {
            assert_eq!(slide.title.to_string(), "Introduction");
        }
        if let SlideProcessingResult::Processed(slide) = &parse_result.slides[1] {
            assert_eq!(slide.title.to_string(), "1. Part One");
        }
        if let SlideProcessingResult::Processed(slide) = &parse_result.slides[2] {
            assert_eq!(slide.title.to_string(), "1.1 Topic A");
        }
        if let SlideProcessingResult::Skipped(slide) = &parse_result.slides[3] {
            assert_eq!(slide.title.to_string(), "Optional Topic"); // Skipped slides don't get counters
        }
        if let SlideProcessingResult::Processed(slide) = &parse_result.slides[4] {
            assert_eq!(slide.title.to_string(), "1.2 Topic B"); // Continues numbering after skip
        }
        if let SlideProcessingResult::Processed(slide) = &parse_result.slides[5] {
            assert_eq!(slide.title.to_string(), "2. Part Two");
        }
        if let SlideProcessingResult::Processed(slide) = &parse_result.slides[6] {
            assert_eq!(slide.title.to_string(), "2.1 Topic C");
        }
    }

    #[test]
    #[allow(clippy::indexing_slicing)]
    fn test_counter_logic_with_skipped_parts() {
        let talk_metadata = TalkMetadata {
            title: "Test Presentation".to_owned(),
            date: Date::today(),
            footer: None,
            head: None,
            typst_preamble: None,
            lang: None,
            default_terminal_cwd: None,
            source_dir: None,
        };

        let slides = vec![
            SlideProcessingResult::Processed(Slide::part("Part One")),
            SlideProcessingResult::Processed(Slide::new("Topic A")),
            SlideProcessingResult::Skipped(Slide::part("Skipped Part")),
            SlideProcessingResult::Processed(Slide::new("Topic B")),
        ];

        let mut parse_result = ParseResult {
            talk_metadata,
            slides,
        };

        add_counters_to_slides(&mut parse_result);

        // Check the titles
        if let SlideProcessingResult::Processed(slide) = &parse_result.slides[0] {
            assert_eq!(slide.title.to_string(), "1. Part One");
        }
        if let SlideProcessingResult::Processed(slide) = &parse_result.slides[1] {
            assert_eq!(slide.title.to_string(), "1.1 Topic A");
        }
        if let SlideProcessingResult::Skipped(slide) = &parse_result.slides[2] {
            assert_eq!(slide.title.to_string(), "Skipped Part"); // Skipped parts don't get counters
        }
        if let SlideProcessingResult::Processed(slide) = &parse_result.slides[3] {
            // This should still be in part context even though the part was skipped
            assert_eq!(slide.title.to_string(), "Topic B");
        }
    }

    /// A deck folder with a cover, one slide, and its own `_preamble.typ`.
    fn deck_with_preamble() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("_cover.md"), "# Deck\n").expect("cover");
        std::fs::write(dir.path().join("01-one.md"), "# One\n\nBody.\n").expect("slide");
        std::fs::write(dir.path().join("_preamble.typ"), "// from the deck\n").expect("preamble");
        dir
    }

    fn settings_for(deck: &Path, extra: &[&str]) -> Settings {
        let mut args = vec!["toboggan-cli".to_owned()];
        args.extend(extra.iter().map(|arg| (*arg).to_owned()));
        args.push(deck.to_string_lossy().into_owned());
        <Settings as clap::Parser>::parse_from(args)
    }

    #[test]
    fn the_preamble_flag_wins_over_the_deck_file() {
        let deck = deck_with_preamble();
        let flag = deck.path().join("other.typ");
        std::fs::write(&flag, "// from the flag\n").expect("write");

        let settings = settings_for(deck.path(), &["--typst-preamble", &flag.to_string_lossy()]);
        let talk = parse_presentation(deck.path(), &settings)
            .expect("parse")
            .to_talk();

        assert_eq!(
            talk.typst_preamble.as_deref(),
            Some("// from the flag\n"),
            "one is what the deck ships with, the other is what this run asked for"
        );
    }

    #[test]
    fn a_blank_preamble_file_means_absent_not_empty() {
        // `touch _preamble.typ` used to be taken at its word: `Some("")` took
        // the replacement branch and emitted a `.typ` with no imports at all,
        // which typst rejects while complaining about a `slide` variable the
        // author never wrote and pointing at a scratch file already deleted.
        let deck = deck_with_preamble();
        let blank = deck.path().join("blank.typ");
        std::fs::write(&blank, "   \n\t\n").expect("write");

        let settings = settings_for(deck.path(), &["--typst-preamble", &blank.to_string_lossy()]);
        let talk = parse_presentation(deck.path(), &settings)
            .expect("parse")
            .to_talk();

        assert_eq!(
            talk.typst_preamble, None,
            "a whitespace-only preamble asks for the generated one, not for none at all"
        );
    }

    #[test]
    fn a_missing_preamble_file_is_an_error_naming_the_path() {
        let deck = deck_with_preamble();
        let missing = deck.path().join("nope.typ");

        let settings = settings_for(
            deck.path(),
            &["--typst-preamble", &missing.to_string_lossy()],
        );
        let error = parse_presentation(deck.path(), &settings)
            .expect_err("a preamble that is not there cannot be used");

        assert!(
            error.to_string().contains("nope.typ"),
            "the error names the file, got: {error}"
        );
    }

    /// `toboggan build -p slides -o public/index.html` in a deck that has a
    /// `public/` used to copy that directory into itself, for ever: the
    /// destination was created first, so it turned up in the source's own
    /// listing. It stopped only at `PATH_MAX` or a full disk, and it destroyed
    /// the author's assets on the way there.
    #[test]
    fn a_destination_inside_the_source_is_refused() {
        let root = tempfile::tempdir().expect("temp dir");
        let source = root.path().join(PUBLIC_DIR);
        std::fs::create_dir_all(&source).expect("create public/");

        // The destination the export would compute for `-o public/index.html`.
        assert!(is_inside(&source, &source.join(PUBLIC_DIR)));
        // And the directory itself, however it is spelled.
        assert!(is_inside(&source, &source));
    }

    /// The everyday export, one directory up, must still copy.
    #[test]
    fn a_destination_beside_the_source_is_copied() {
        let root = tempfile::tempdir().expect("temp dir");
        let source = root.path().join(PUBLIC_DIR);
        std::fs::create_dir_all(source.join("images")).expect("create public/images");
        std::fs::write(source.join("logo.png"), b"png").expect("write logo");
        std::fs::write(source.join("images/diagram.svg"), b"svg").expect("write diagram");

        let destination = root.path().join("out").join(PUBLIC_DIR);
        assert!(!is_inside(&source, &destination));

        let count = copy_dir(&source, &destination).expect("copy assets");
        assert_eq!(count, 2, "both files, at both depths");
        assert!(destination.join("images/diagram.svg").is_file());
    }
}
