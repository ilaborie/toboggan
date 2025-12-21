#![allow(clippy::result_large_err)]

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use toboggan_core::{Date, Slide, Talk};
use tracing::debug;

pub mod error;
pub use self::error::{Result, TobogganCliError};

pub mod parser;
use parser::FolderParser;

pub mod output;

mod settings;
pub use self::settings::*;

pub mod display;

pub mod stats;

#[derive(Debug, Clone)]
pub enum SlideProcessingResult {
    Processed(Slide),
    Skipped(Slide),
    Ignored(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct TalkMetadata {
    pub title: String,
    pub date: Date,
    pub footer: Option<String>,
    pub head: Option<String>,
}

impl Default for TalkMetadata {
    fn default() -> Self {
        Self {
            title: "Unknown Talk".to_string(),
            date: Date::today(),
            footer: None,
            head: None,
        }
    }
}

#[derive(Debug, Clone)]
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

        for slide_result in &self.slides {
            if let SlideProcessingResult::Processed(slide) = slide_result {
                talk.slides.push(slide.clone());
            }
        }

        talk
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
            SlideProcessingResult::Skipped(slide) => {
                if slide.kind == toboggan_core::SlideKind::Part {
                    in_part = false;
                }
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

    let input = validate_input(settings.input.as_ref())?;
    let parse_result = parse_presentation(input, settings)?;
    display_results(&parse_result, settings)?;

    if let Some(output) = &settings.output {
        write_output(&parse_result, output, settings)?;
    } else {
        display::suggest_output_file(&mut std::io::stdout())?;
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

fn parse_presentation(input: &Path, settings: &Settings) -> Result<ParseResult> {
    debug!("Processing folder-based talk from {}", input.display());

    let parser = FolderParser::new(input.to_path_buf(), settings.theme.clone())?;
    let mut parse_result = parser.parse(settings.title.clone(), settings.date)?;

    if !settings.no_counter {
        add_counters_to_slides(&mut parse_result);
    }

    Ok(parse_result)
}

fn display_results(parse_result: &ParseResult, settings: &Settings) -> Result<()> {
    let display_formatter = display::DisplayFormatter::new();
    display_formatter.display_results(parse_result, &mut std::io::stdout())?;

    if !settings.no_stats {
        let stats = stats::PresentationStats::from_parse_result(
            parse_result,
            settings.wpm,
            !settings.exclude_notes_from_duration,
        );
        stats.display(
            &mut std::io::stdout(),
            display::DisplayConfig::should_use_colors(),
        )?;
    }

    Ok(())
}

#[allow(clippy::print_stderr)]
fn write_output(parse_result: &ParseResult, output: &Path, settings: &Settings) -> Result<()> {
    let format = settings.resolve_format();
    let talk = parse_result.to_talk();
    let serialized = output::serialize_talk(&talk, format)?;

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

    Ok(())
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

#[allow(clippy::print_stdout)]
fn list_available_themes() {
    println!("{}", include_str!("available_themes.txt"));
}

fn parse_date_string(date_str: &str) -> Result<Date> {
    let regex = regex::Regex::new(r"^(\d{4})-(\d{1,2})-(\d{1,2})$")?;

    if let Some(caps) = regex.captures(date_str) {
        let year =
            caps[1]
                .parse::<i16>()
                .map_err(|source| TobogganCliError::InvalidDateComponent {
                    component: "year".to_string(),
                    value: caps[1].to_string(),
                    source,
                })?;
        let month =
            caps[2]
                .parse::<i8>()
                .map_err(|source| TobogganCliError::InvalidDateComponent {
                    component: "month".to_string(),
                    value: caps[2].to_string(),
                    source,
                })?;
        let day =
            caps[3]
                .parse::<i8>()
                .map_err(|source| TobogganCliError::InvalidDateComponent {
                    component: "day".to_string(),
                    value: caps[3].to_string(),
                    source,
                })?;
        let date = Date::new(year, month, day).map_err(|_| TobogganCliError::InvalidDate {
            year,
            month,
            day,
        })?;

        Ok(date)
    } else {
        Err(TobogganCliError::InvalidDateFormat {
            input: date_str.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use toboggan_core::Slide;

    use super::*;

    fn create_test_parse_result_for_counter() -> ParseResult {
        let talk_metadata = TalkMetadata {
            title: "Test Presentation".to_string(),
            date: toboggan_core::Date::today(),
            footer: None,
            head: None,
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
            title: "Test Presentation".to_string(),
            date: toboggan_core::Date::today(),
            footer: None,
            head: None,
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
}
