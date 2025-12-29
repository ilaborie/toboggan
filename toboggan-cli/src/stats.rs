use std::collections::HashMap;
use std::io::Write;
use std::time::Duration;

use comfy_table::presets::UTF8_HORIZONTAL_ONLY;
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table};
use owo_colors::OwoColorize;
use toboggan_core::SlideKind;
use toboggan_stats::{PresentationStats as CoreStats, SlideStats};

use crate::{ParseResult, SlideProcessingResult};

/// Presentation statistics with display capabilities
#[derive(Debug, Clone, Default)]
pub struct PresentationStats {
    /// Core statistics computed by toboggan-stats
    pub core: CoreStats,
    /// Words per minute for duration calculation
    pub wpm: u16,
    /// Whether to include notes in duration calculation
    pub include_notes_in_duration: bool,
}

impl PresentationStats {
    #[must_use]
    pub fn new(wpm: u16, include_notes_in_duration: bool) -> Self {
        Self {
            core: CoreStats::default(),
            wpm,
            include_notes_in_duration,
        }
    }

    /// Calculate statistics from parse results
    #[must_use]
    pub fn from_parse_result(
        parse_result: &ParseResult,
        wpm: u16,
        include_notes_in_duration: bool,
    ) -> Self {
        let mut stats = Self::new(wpm, include_notes_in_duration);
        let mut current_part: Option<String> = None;
        let mut has_slides_before_first_part = false;

        for slide_result in &parse_result.slides {
            if let SlideProcessingResult::Processed(slide) = slide_result {
                if slide.kind == SlideKind::Part {
                    stats.core.total_parts += 1;
                    let part_name = slide.title.to_string();
                    current_part = Some(part_name.clone());
                    stats.core.part_order.push(part_name.clone());
                    stats.core.slides_per_part.insert(part_name.clone(), 0);
                    stats.core.words_per_part.insert(part_name.clone(), 0);
                    stats.core.bullets_per_part.insert(part_name.clone(), 0);
                    stats.core.images_per_part.insert(part_name.clone(), 0);
                    stats.core.steps_per_part.insert(part_name.clone(), 0);
                    stats.core.notes_words_per_part.insert(part_name, 0);
                } else {
                    stats.core.total_slides += 1;

                    // Compute slide stats with counter stripping for title
                    let slide_stats = SlideStats::from_slide_with_options(slide, true);

                    stats.core.total_words += slide_stats.words;
                    stats.core.total_bullets += slide_stats.bullets;
                    stats.core.total_images += slide_stats.images;
                    stats.core.total_steps += slide_stats.steps;
                    stats.core.total_notes_words += slide_stats.notes_words;

                    // Add to current part or create implicit "Introduction"
                    if current_part.is_none() && !has_slides_before_first_part {
                        has_slides_before_first_part = true;
                        let intro_part = "(Introduction)".to_string();
                        stats.core.part_order.insert(0, intro_part.clone());
                        stats.core.slides_per_part.insert(intro_part.clone(), 0);
                        stats.core.words_per_part.insert(intro_part.clone(), 0);
                        stats.core.bullets_per_part.insert(intro_part.clone(), 0);
                        stats.core.images_per_part.insert(intro_part.clone(), 0);
                        stats.core.steps_per_part.insert(intro_part.clone(), 0);
                        stats
                            .core
                            .notes_words_per_part
                            .insert(intro_part.clone(), 0);
                        current_part = Some(intro_part);
                    }

                    if let Some(ref part_name) = current_part {
                        *stats
                            .core
                            .slides_per_part
                            .get_mut(part_name)
                            .unwrap_or(&mut 0) += 1;
                        *stats
                            .core
                            .words_per_part
                            .get_mut(part_name)
                            .unwrap_or(&mut 0) += slide_stats.words;
                        *stats
                            .core
                            .bullets_per_part
                            .get_mut(part_name)
                            .unwrap_or(&mut 0) += slide_stats.bullets;
                        *stats
                            .core
                            .images_per_part
                            .get_mut(part_name)
                            .unwrap_or(&mut 0) += slide_stats.images;
                        *stats
                            .core
                            .steps_per_part
                            .get_mut(part_name)
                            .unwrap_or(&mut 0) += slide_stats.steps;
                        *stats
                            .core
                            .notes_words_per_part
                            .get_mut(part_name)
                            .unwrap_or(&mut 0) += slide_stats.notes_words;
                    }
                }
            }
        }

        stats
    }

    #[must_use]
    pub fn duration_estimates(&self) -> toboggan_stats::DurationEstimate {
        self.core
            .duration_estimates(self.wpm, self.include_notes_in_duration)
    }

    fn calculate_adjusted_percentages(&self) -> HashMap<String, f64> {
        let mut result = HashMap::new();

        // Calculate effective total including notes if they're included in duration
        let effective_total: usize = if self.include_notes_in_duration {
            self.core.words_per_part.values().sum::<usize>()
                + self.core.notes_words_per_part.values().sum::<usize>()
        } else {
            self.core.words_per_part.values().sum::<usize>()
        };

        if effective_total == 0 {
            return result;
        }

        // Calculate raw percentages and round to 1 decimal place
        let parts_data = self
            .core
            .slides_per_part
            .keys()
            .map(|name| {
                let words = self.core.words_per_part.get(name).unwrap_or(&0);
                let part_total = if self.include_notes_in_duration {
                    words + self.core.notes_words_per_part.get(name).unwrap_or(&0)
                } else {
                    *words
                };
                #[allow(clippy::cast_precision_loss)]
                let raw_percentage = (part_total as f64 / effective_total as f64) * 100.0;
                let rounded = (raw_percentage * 10.0).round() / 10.0;
                (name.clone(), part_total, rounded)
            })
            .collect::<Vec<_>>();

        // Calculate total and adjustment needed
        let total_percentage: f64 = parts_data.iter().map(|(_, _, pct)| pct).sum();
        let adjustment = ((100.0 - total_percentage) * 10.0).round() / 10.0;

        // Find the largest part by word count for adjustment
        let largest_part_name = parts_data
            .iter()
            .max_by_key(|(_, words, _)| words)
            .map(|(name, _, _)| name.clone());

        // Apply adjustment to the largest part while keeping original order
        for (name, _, mut percentage) in parts_data {
            if Some(&name) == largest_part_name.as_ref() {
                percentage += adjustment;
            }
            result.insert(name, percentage);
        }

        result
    }

    /// Display comprehensive statistics
    pub fn display<W: Write>(&self, writer: &mut W, use_colors: bool) -> std::io::Result<()> {
        let duration = self.duration_estimates();

        writeln!(writer)?;
        Self::write_header("📊 Presentation Statistics", writer, use_colors)?;

        // Overview
        self.write_overview(&duration, writer, use_colors)?;

        // Part breakdown if we have parts
        if self.core.total_parts > 0 {
            writeln!(writer)?;
            writeln!(writer)?;
            self.write_part_breakdown(writer, use_colors)?;
        }

        // Duration scenarios
        writeln!(writer)?;
        writeln!(writer)?;
        self.write_duration_scenarios(&duration, writer, use_colors)?;

        // Recommendations
        if let Some(recommendations) = self.generate_recommendations() {
            writeln!(writer)?;
            Self::write_recommendations(&recommendations, writer, use_colors)?;
        }

        Ok(())
    }

    fn write_header<W: Write>(text: &str, writer: &mut W, use_colors: bool) -> std::io::Result<()> {
        if use_colors {
            writeln!(writer, "{}", text.bold().blue())
        } else {
            writeln!(writer, "{text}")
        }
    }

    fn write_overview<W: Write>(
        &self,
        duration: &toboggan_stats::DurationEstimate,
        writer: &mut W,
        use_colors: bool,
    ) -> std::io::Result<()> {
        let total_duration = duration.custom + duration.image_time;
        let minutes = total_duration.as_secs() / 60;
        let seconds = total_duration.as_secs() % 60;

        if use_colors {
            writeln!(writer, "{}:", "Overview".bold())?;
            writeln!(
                writer,
                "  • Total slides: {}",
                self.core.total_slides.to_string().cyan()
            )?;
            if self.core.total_parts > 0 {
                writeln!(
                    writer,
                    "  • Parts: {}",
                    self.core.total_parts.to_string().cyan()
                )?;
            }
            writeln!(
                writer,
                "  • Words: {}",
                self.core.total_words.to_string().cyan()
            )?;
            if self.core.total_notes_words > 0 {
                writeln!(
                    writer,
                    "  • Notes words: {}{}",
                    self.core.total_notes_words.to_string().cyan(),
                    if self.include_notes_in_duration {
                        " (included in duration)".dimmed()
                    } else {
                        " (not included in duration)".dimmed()
                    }
                )?;
            }
            writeln!(
                writer,
                "  • Bullet points: {}",
                self.core.total_bullets.to_string().cyan()
            )?;
            writeln!(
                writer,
                "  • Images: {}",
                self.core.total_images.to_string().cyan()
            )?;
            writeln!(
                writer,
                "  • Estimated duration: {}:{} {} ({} WPM)",
                minutes.to_string().green().bold(),
                format!("{seconds:02}").green().bold(),
                "minutes".green(),
                self.wpm.to_string().yellow()
            )?;
        } else {
            writeln!(writer, "Overview:")?;
            writeln!(writer, "  • Total slides: {}", self.core.total_slides)?;
            if self.core.total_parts > 0 {
                writeln!(writer, "  • Parts: {}", self.core.total_parts)?;
            }
            writeln!(writer, "  • Words: {}", self.core.total_words)?;
            if self.core.total_notes_words > 0 {
                writeln!(
                    writer,
                    "  • Notes words: {} ({})",
                    self.core.total_notes_words,
                    if self.include_notes_in_duration {
                        "included in duration"
                    } else {
                        "not included in duration"
                    }
                )?;
            }
            writeln!(writer, "  • Bullet points: {}", self.core.total_bullets)?;
            writeln!(writer, "  • Images: {}", self.core.total_images)?;
            writeln!(
                writer,
                "  • Estimated duration: {}:{:02} minutes ({} WPM)",
                minutes, seconds, self.wpm
            )?;
        }
        Ok(())
    }

    fn write_part_breakdown<W: Write>(
        &self,
        writer: &mut W,
        use_colors: bool,
    ) -> std::io::Result<()> {
        if use_colors {
            writeln!(writer, "{}:", "Part Breakdown".bold())?;
        } else {
            writeln!(writer, "Part Breakdown:")?;
        }

        // Calculate adjusted percentages that sum to exactly 100%
        let adjusted_percentages = self.calculate_adjusted_percentages();

        // Create table
        let mut table = Table::new();
        table.load_preset(UTF8_HORIZONTAL_ONLY);

        // Add headers
        if use_colors {
            table.set_header(vec![
                Cell::new("Part").add_attribute(Attribute::Bold),
                Cell::new("Slides").add_attribute(Attribute::Bold),
                Cell::new("Words").add_attribute(Attribute::Bold),
                Cell::new("Percentage").add_attribute(Attribute::Bold),
                Cell::new("Duration").add_attribute(Attribute::Bold),
            ]);
        } else {
            table.set_header(vec!["Part", "Slides", "Words", "Percentage", "Duration"]);
        }

        // Set column alignments - numbers to the right
        if let Some(column) = table.column_mut(1) {
            column.set_cell_alignment(CellAlignment::Right);
        }
        if let Some(column) = table.column_mut(2) {
            column.set_cell_alignment(CellAlignment::Right);
        }
        if let Some(column) = table.column_mut(3) {
            column.set_cell_alignment(CellAlignment::Right);
        }
        if let Some(column) = table.column_mut(4) {
            column.set_cell_alignment(CellAlignment::Right);
        }

        // Iterate over parts in their original order
        for part_name in &self.core.part_order {
            let slide_count = self.core.slides_per_part.get(part_name).unwrap_or(&0);
            let words = self.core.words_per_part.get(part_name).unwrap_or(&0);
            let notes_words = if self.include_notes_in_duration {
                self.core.notes_words_per_part.get(part_name).unwrap_or(&0)
            } else {
                &0
            };
            let percentage = adjusted_percentages.get(part_name).unwrap_or(&0.0);

            #[allow(clippy::cast_precision_loss)]
            let total_words = (*words + *notes_words) as f64;
            let part_duration = Duration::from_secs_f64((total_words / f64::from(self.wpm)) * 60.0);
            let minutes = part_duration.as_secs() / 60;
            let seconds = part_duration.as_secs() % 60;

            if use_colors {
                table.add_row(vec![
                    Cell::new(part_name).fg(Color::Yellow),
                    Cell::new(slide_count.to_string()).fg(Color::Cyan),
                    Cell::new(words.to_string()).fg(Color::Cyan),
                    Cell::new(format!("{percentage:.1}%")).fg(Color::Magenta),
                    Cell::new(format!("{minutes}:{seconds:02}")).fg(Color::Green),
                ]);
            } else {
                table.add_row(vec![
                    part_name,
                    &slide_count.to_string(),
                    &words.to_string(),
                    &format!("{percentage:.1}%"),
                    &format!("{minutes}:{seconds:02}"),
                ]);
            }
        }

        write!(writer, "{table}")?;
        Ok(())
    }

    fn write_duration_scenarios<W: Write>(
        &self,
        duration: &toboggan_stats::DurationEstimate,
        writer: &mut W,
        use_colors: bool,
    ) -> std::io::Result<()> {
        if use_colors {
            writeln!(writer, "{}:", "Duration Scenarios".bold())?;
        } else {
            writeln!(writer, "Duration Scenarios:")?;
        }

        // Create table
        let mut table = Table::new();
        table.load_preset(UTF8_HORIZONTAL_ONLY);

        // Add headers
        if use_colors {
            table.set_header(vec![
                Cell::new("Speaking Rate").add_attribute(Attribute::Bold),
                Cell::new("Duration").add_attribute(Attribute::Bold),
            ]);
        } else {
            table.set_header(vec!["Speaking Rate", "Duration"]);
        }

        // Set column alignment - duration to the right
        if let Some(column) = table.column_mut(1) {
            column.set_cell_alignment(CellAlignment::Right);
        }

        let scenarios = [
            ("Slow speaker (110 WPM)", &duration.slow),
            ("Normal speaker (150 WPM)", &duration.normal),
            ("Fast speaker (170 WPM)", &duration.fast),
        ];

        for (label, dur) in scenarios {
            let total = *dur + duration.image_time;
            let minutes = total.as_secs() / 60;
            let seconds = total.as_secs() % 60;

            if use_colors {
                table.add_row(vec![
                    Cell::new(label).fg(Color::Yellow),
                    Cell::new(format!("{minutes}:{seconds:02} minutes")).fg(Color::Green),
                ]);
            } else {
                table.add_row(vec![label, &format!("{minutes}:{seconds:02} minutes")]);
            }
        }

        // Add image viewing time if present
        if self.core.total_images > 0 {
            let img_minutes = duration.image_time.as_secs() / 60;
            let img_seconds = duration.image_time.as_secs() % 60;
            if use_colors {
                table.add_row(vec![
                    Cell::new("Image viewing time").fg(Color::Cyan),
                    Cell::new(format!("+{img_minutes}:{img_seconds:02} minutes")).fg(Color::Cyan),
                ]);
            } else {
                table.add_row(vec![
                    "Image viewing time",
                    &format!("+{img_minutes}:{img_seconds:02} minutes"),
                ]);
            }
        }

        write!(writer, "{table}")?;
        Ok(())
    }

    fn write_recommendations<W: Write>(
        recommendations: &[String],
        writer: &mut W,
        use_colors: bool,
    ) -> std::io::Result<()> {
        if use_colors {
            writeln!(writer, "{}:", "Recommendations".bold())?;
        } else {
            writeln!(writer, "Recommendations:")?;
        }

        for rec in recommendations {
            if use_colors {
                writeln!(writer, "  💡 {}", rec.bright_yellow())?;
            } else {
                writeln!(writer, "  • {rec}")?;
            }
        }
        Ok(())
    }

    fn generate_recommendations(&self) -> Option<Vec<String>> {
        let mut recommendations = Vec::new();
        let duration = self.duration_estimates();
        let total_minutes = (duration.custom + duration.image_time).as_secs() / 60;

        // Duration recommendations
        if total_minutes > 50 {
            recommendations
                .push("Consider splitting this into multiple shorter presentations".to_string());
        } else if total_minutes < 2 {
            recommendations.push(
                "This presentation might be too short - consider adding more content".to_string(),
            );
        }

        // Part balance recommendations
        if self.core.total_parts > 1 {
            let mut max_percentage = 0.0;
            let mut max_part = String::new();

            for (part_name, &words) in &self.core.words_per_part {
                #[allow(clippy::cast_precision_loss)]
                let percentage = (words as f64 / self.core.total_words as f64) * 100.0;
                if percentage > max_percentage {
                    max_percentage = percentage;
                    max_part.clone_from(part_name);
                }
            }

            if max_percentage > 50.0 {
                recommendations.push(format!(
                    "'{max_part}' contains {max_percentage:.1}% of content - consider splitting"
                ));
            }
        }

        // Content density recommendations
        if self.core.total_slides > 0 {
            let avg_words_per_slide = self.core.total_words / self.core.total_slides;
            if avg_words_per_slide > 100 {
                recommendations.push(
                    "High word density - consider more slides with less text each".to_string(),
                );
            } else if avg_words_per_slide < 20 {
                recommendations.push(
                    "Low word density - slides might benefit from more detailed content"
                        .to_string(),
                );
            }
        }

        if recommendations.is_empty() {
            None
        } else {
            Some(recommendations)
        }
    }
}

#[cfg(test)]
mod tests {
    use toboggan_core::{Content, Slide, Style};

    use super::*;
    use crate::{ParseResult, SlideProcessingResult, TalkMetadata};

    #[test]
    fn test_statistics_calculation() {
        let talk_metadata = TalkMetadata {
            title: "Test Talk".to_string(),
            date: toboggan_core::Date::today(),
            footer: None,
            head: None,
        };

        let intro_slide = Slide::new("Introduction").with_body(Content::Text {
            text: "Hello world this is a test".to_string(),
        });

        let part_slide = Slide::part("Part One").with_body(Content::Text {
            text: "Part introduction".to_string(),
        });

        let slides = vec![
            SlideProcessingResult::Processed(intro_slide),
            SlideProcessingResult::Processed(part_slide),
        ];

        let parse_result = ParseResult {
            talk_metadata,
            slides,
        };

        let stats = PresentationStats::from_parse_result(&parse_result, 150, false);

        assert_eq!(stats.core.total_slides, 1); // Only non-part slides
        assert_eq!(stats.core.total_parts, 1);
        assert!(stats.core.total_words > 0);
        // Verify intro slide is counted in the breakdown
        let total_in_parts: usize = stats.core.slides_per_part.values().sum();
        assert_eq!(
            total_in_parts, stats.core.total_slides,
            "All slides should be in parts"
        );
    }

    #[test]
    fn test_diagram_slide_counting() {
        let talk_metadata = TalkMetadata {
            title: "Test Talk".to_string(),
            date: toboggan_core::Date::today(),
            footer: None,
            head: None,
        };

        // Simulate a diagram slide with counter: title="3.5 Diagram" body=SVG
        let mut diagram_slide = Slide::new("Diagram").with_body(Content::Html {
            raw: r#"<svg width="100" height="100"><circle cx="50" cy="50" r="40"/><text x="50" y="50">Label</text></svg>"#.to_string(),
            style: Style::default(),
            alt: None,
        });
        // Simulate counter being added (like add_counters_to_slides does)
        diagram_slide.title = "3.5 Diagram".into();

        let part_slide = Slide::part("Part 3");

        let slides = vec![
            SlideProcessingResult::Processed(part_slide),
            SlideProcessingResult::Processed(diagram_slide),
        ];

        let parse_result = ParseResult {
            talk_metadata,
            slides,
        };

        let stats = PresentationStats::from_parse_result(&parse_result, 150, false);

        assert_eq!(stats.core.total_slides, 1, "Should have 1 slide");
        assert_eq!(stats.core.total_images, 1, "Should count SVG as 1 image");
        // Counter "3.5 " is stripped, only "Diagram" counted
        assert_eq!(
            stats.core.total_words, 1,
            "Title '3.5 Diagram' counts as 1 word (counter stripped, text inside SVG excluded)"
        );
    }

    #[test]
    fn test_notes_not_double_counted_in_duration() {
        let talk_metadata = TalkMetadata {
            title: "Test Talk".to_string(),
            date: toboggan_core::Date::today(),
            footer: None,
            head: None,
        };

        // Create a slide with content words (2 in title + 6 in body = 8 total) and 4 notes words
        let slide = Slide::new("Test Slide")
            .with_body(Content::Text {
                text: "one two three four five six".to_string(),
            })
            .with_notes(Content::Text {
                text: "note1 note2 note3 note4".to_string(),
            });

        let slides = vec![SlideProcessingResult::Processed(slide)];

        let parse_result = ParseResult {
            talk_metadata,
            slides,
        };

        // Test with notes included in duration
        let stats = PresentationStats::from_parse_result(&parse_result, 150, true);

        assert_eq!(
            stats.core.total_words, 8,
            "total_words should count title + body (2 + 6 = 8)"
        );
        assert_eq!(
            stats.core.total_notes_words, 4,
            "notes should be tracked separately"
        );

        let duration = stats.duration_estimates();
        // Duration should be based on 8 + 4 = 12 words at 150 WPM = 0.08 minutes = 4.8 seconds
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let expected_seconds = ((12.0 / 150.0) * 60.0) as u64;
        assert_eq!(
            duration.custom.as_secs(),
            expected_seconds,
            "duration should include both content and notes words exactly once"
        );

        // Test with notes NOT included in duration
        let stats_no_notes = PresentationStats::from_parse_result(&parse_result, 150, false);
        let duration_no_notes = stats_no_notes.duration_estimates();
        // Duration should be based on 8 words only at 150 WPM = 0.0533 minutes = 3.2 seconds
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let expected_seconds_no_notes = ((8.0 / 150.0) * 60.0) as u64;
        assert_eq!(
            duration_no_notes.custom.as_secs(),
            expected_seconds_no_notes,
            "duration should only include content words when notes are excluded"
        );
    }

    #[test]
    fn test_html_content_analysis() {
        let talk_metadata = TalkMetadata {
            title: "Test Talk".to_string(),
            date: toboggan_core::Date::today(),
            footer: None,
            head: None,
        };

        let slide = Slide::new("Title").with_body(Content::Html {
            raw: r"<ul><li>Item 1</li><li>Item 2</li><li>Item 3</li></ul>".to_string(),
            style: Style::default(),
            alt: None,
        });

        let slides = vec![SlideProcessingResult::Processed(slide)];
        let parse_result = ParseResult {
            talk_metadata,
            slides,
        };

        let stats = PresentationStats::from_parse_result(&parse_result, 150, false);

        assert_eq!(stats.core.total_bullets, 3, "Should count 3 list items");
    }

    #[test]
    fn test_step_counting() {
        let talk_metadata = TalkMetadata {
            title: "Test Talk".to_string(),
            date: toboggan_core::Date::today(),
            footer: None,
            head: None,
        };

        let slide = Slide::new("Title").with_body(Content::Html {
            raw: r#"<div class="step">Step 1</div><div class="step">Step 2</div><div class="step">Step 3</div>"#.to_string(),
            style: Style::default(),
            alt: None,
        });

        let slides = vec![SlideProcessingResult::Processed(slide)];
        let parse_result = ParseResult {
            talk_metadata,
            slides,
        };

        let stats = PresentationStats::from_parse_result(&parse_result, 150, false);

        assert_eq!(stats.core.total_steps, 3, "Should count 3 steps");
    }
}
