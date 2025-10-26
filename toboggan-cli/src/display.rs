use std::io::{IsTerminal, Write};

use owo_colors::OwoColorize;
use toboggan_core::SlideKind;

use crate::{ParseResult, SlideProcessingResult};

#[derive(Debug, Clone)]
pub struct DisplayConfig {
    pub use_colors: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            use_colors: Self::should_use_colors(),
        }
    }
}

impl DisplayConfig {
    #[must_use]
    pub fn should_use_colors() -> bool {
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }

        std::io::stdout().is_terminal()
    }

    #[must_use]
    pub fn no_colors() -> Self {
        Self { use_colors: false }
    }

    #[must_use]
    pub fn with_colors() -> Self {
        Self { use_colors: true }
    }
}

pub struct DisplayFormatter {
    config: DisplayConfig,
}

impl DisplayFormatter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: DisplayConfig::default(),
        }
    }

    #[must_use]
    pub fn with_config(config: DisplayConfig) -> Self {
        Self { config }
    }

    pub fn display_results<W: Write>(
        &self,
        results: &ParseResult,
        writer: &mut W,
    ) -> std::io::Result<()> {
        self.display_header(results, writer)?;
        self.display_slides(results, writer)?;
        self.display_summary(results, writer)?;
        Ok(())
    }

    fn display_header<W: Write>(
        &self,
        results: &ParseResult,
        writer: &mut W,
    ) -> std::io::Result<()> {
        write!(writer, "Talk: ")?;
        if self.config.use_colors {
            write!(writer, "{}", results.talk_metadata.title.bold().blue())?;
        } else {
            write!(writer, "{}", results.talk_metadata.title)?;
        }
        writeln!(writer)?;
        Ok(())
    }

    fn display_slides<W: Write>(
        &self,
        results: &ParseResult,
        writer: &mut W,
    ) -> std::io::Result<()> {
        for slide_result in &results.slides {
            self.display_slide_result(slide_result, writer)?;
        }
        Ok(())
    }

    fn display_slide_result<W: Write>(
        &self,
        slide_result: &SlideProcessingResult,
        writer: &mut W,
    ) -> std::io::Result<()> {
        match slide_result {
            SlideProcessingResult::Processed(slide) => {
                let title = slide.title.to_string();

                let indent = match slide.kind {
                    SlideKind::Part | SlideKind::Cover => "",
                    SlideKind::Standard => "  ",
                };
                write!(writer, "{indent}")?;
                self.write_slide_title(&title, slide.kind, false, writer)?;
                writeln!(writer)?;
            }
            SlideProcessingResult::Skipped(slide) => {
                let title = slide.title.to_string();

                let indent = match slide.kind {
                    SlideKind::Part => "",
                    _ => "  ", // 2 spaces for standard and cover slides
                };
                write!(writer, "{indent}")?;
                self.write_status_indicator("[SKIP]", StatusColor::Yellow, writer)?;
                write!(writer, " ")?;
                self.write_slide_title(&title, slide.kind, true, writer)?;
                writeln!(writer)?;
            }
            SlideProcessingResult::Ignored(description) => {
                self.write_status_indicator("[IGNORE]", StatusColor::Gray, writer)?;
                write!(writer, " ")?;
                if self.config.use_colors {
                    write!(writer, "{}", description.dimmed())?;
                } else {
                    write!(writer, "{description}")?;
                }
                writeln!(writer)?;
            }
            SlideProcessingResult::Error(description) => {
                self.write_status_indicator("[ERROR]", StatusColor::Red, writer)?;
                write!(writer, " {description}")?;
                writeln!(writer)?;
            }
        }
        Ok(())
    }

    fn write_slide_title<W: Write>(
        &self,
        title: &str,
        kind: SlideKind,
        is_skipped: bool,
        writer: &mut W,
    ) -> std::io::Result<()> {
        if self.config.use_colors {
            match kind {
                SlideKind::Cover => {
                    if is_skipped {
                        write!(writer, "{}", title.dimmed().blue())?;
                    } else {
                        write!(writer, "{}", title.bold().blue())?;
                    }
                }
                SlideKind::Part => {
                    if is_skipped {
                        write!(writer, "{}", title.dimmed().green())?;
                    } else {
                        write!(writer, "{}", title.bold().green())?;
                    }
                }
                SlideKind::Standard => {
                    if is_skipped {
                        write!(writer, "{}", title.dimmed())?;
                    } else {
                        write!(writer, "{title}")?;
                    }
                }
            }
        } else {
            write!(writer, "{title}")?;
        }
        Ok(())
    }

    fn write_status_indicator<W: Write>(
        &self,
        text: &str,
        color: StatusColor,
        writer: &mut W,
    ) -> std::io::Result<()> {
        if self.config.use_colors {
            match color {
                StatusColor::Green => {
                    write!(writer, "{}", text.green())?;
                }
                StatusColor::Yellow => {
                    write!(writer, "{}", text.yellow())?;
                }
                StatusColor::Gray => {
                    write!(writer, "{}", text.bright_black())?;
                }
                StatusColor::Red => {
                    write!(writer, "{}", text.red())?;
                }
            }
        } else {
            write!(writer, "{text}")?;
        }
        Ok(())
    }

    fn display_summary<W: Write>(
        &self,
        results: &ParseResult,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let stats = results.stats();

        if stats.total() == 0 {
            writeln!(writer, "\nNo slides found.")?;
            return Ok(());
        }

        writeln!(writer)?; // Empty line before summary

        let summary_parts = vec![
            (stats.processed, "processed", StatusColor::Green),
            (stats.skipped, "skipped", StatusColor::Yellow),
            (stats.ignored, "ignored", StatusColor::Gray),
            (stats.errors, "errors", StatusColor::Red),
        ];

        let summary_parts: Vec<_> = summary_parts
            .into_iter()
            .filter(|(count, _, _)| *count > 0)
            .collect();

        if !summary_parts.is_empty() {
            write!(writer, "Summary: ")?;
            for (i, (count, label, color)) in summary_parts.iter().enumerate() {
                if i > 0 {
                    write!(writer, ", ")?;
                }
                let text = format!("{count} {label}");
                self.write_status_indicator(&text, *color, writer)?;
            }
            writeln!(writer)?;
        }

        Ok(())
    }
}

impl Default for DisplayFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
enum StatusColor {
    Green,
    Yellow,
    Gray,
    Red,
}

pub fn suggest_output_file<W: Write>(writer: &mut W) -> std::io::Result<()> {
    writeln!(writer)?;
    writeln!(
        writer,
        "💡 Tip: Use '-o filename.toml' to save the presentation to a file."
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use toboggan_core::Slide;

    use super::*;
    use crate::TalkMetadata;

    fn create_test_results() -> ParseResult {
        let talk_metadata = TalkMetadata {
            title: "Test Presentation".to_string(),
            date: toboggan_core::Date::today(),
            footer: None,
        };

        let slides = vec![
            SlideProcessingResult::Processed(Slide::new("Introduction")),
            SlideProcessingResult::Processed(Slide::part("Overview")),
            SlideProcessingResult::Processed(Slide::new("First Topic")),
            SlideProcessingResult::Skipped(Slide::new("Optional Content")),
            SlideProcessingResult::Processed(Slide::new("Second Topic")),
            SlideProcessingResult::Ignored("Invalid file format".to_string()),
            SlideProcessingResult::Error("Parse error in slide".to_string()),
        ];

        ParseResult {
            talk_metadata,
            slides,
        }
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn test_display_results_no_colors() {
        let results = create_test_results();
        let config = DisplayConfig::no_colors();
        let formatter = DisplayFormatter::with_config(config);
        let mut output = Cursor::new(Vec::new());

        formatter
            .display_results(&results, &mut output)
            .expect("Failed to display results");

        let output_str =
            String::from_utf8(output.into_inner()).expect("Failed to convert output to UTF-8");
        assert!(output_str.contains("Talk: Test Presentation"));
        assert!(output_str.contains("Introduction"));
        assert!(output_str.contains("[SKIP]"));
        assert!(output_str.contains("[IGNORE]"));
        assert!(output_str.contains("[ERROR]"));
        assert!(output_str.contains("Summary:"));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn test_display_results_basic() {
        let results = create_test_results();
        let config = DisplayConfig { use_colors: false };
        let formatter = DisplayFormatter::with_config(config);
        let mut output = Cursor::new(Vec::new());

        formatter
            .display_results(&results, &mut output)
            .expect("Failed to display results");

        let output_str =
            String::from_utf8(output.into_inner()).expect("Failed to convert output to UTF-8");
        assert!(output_str.contains("Talk: Test Presentation"));
        assert!(output_str.contains("Introduction"));
        assert!(output_str.contains("Overview"));
        assert!(output_str.contains("First Topic"));
        assert!(output_str.contains("[SKIP] Optional Content"));
        assert!(output_str.contains("Second Topic"));
        assert!(output_str.contains("Summary:"));
    }

    #[test]
    fn test_stats_calculation() {
        let results = create_test_results();
        let stats = results.stats();

        assert_eq!(stats.processed, 4);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.ignored, 1);
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.total(), 7);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn test_suggest_output_file() {
        let mut output = Cursor::new(Vec::new());
        suggest_output_file(&mut output).expect("Failed to suggest output file");

        let output_str =
            String::from_utf8(output.into_inner()).expect("Failed to convert output to UTF-8");
        assert!(output_str.contains("Tip:"));
        assert!(output_str.contains("-o filename.toml"));
    }
}
