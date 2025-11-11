use std::collections::HashMap;
use std::io::Write;
use std::sync::LazyLock;
use std::time::Duration;

use comfy_table::presets::UTF8_HORIZONTAL_ONLY;
use comfy_table::{Attribute, Cell, CellAlignment, Color, Table};
use owo_colors::OwoColorize;
use regex::Regex;
use toboggan_core::{Content, SlideKind};

use crate::{ParseResult, SlideProcessingResult};

#[allow(clippy::expect_used)]
static BULLET_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*[-*+•]\s").expect("bullet regex should be valid"));

#[allow(clippy::expect_used)]
static NUMBERED_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*\d+\.\s").expect("numbered regex should be valid"));

#[allow(clippy::expect_used)]
static HTML_TAG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]*>").expect("HTML tag regex should be valid"));

#[allow(clippy::expect_used)]
static IMG_TAG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<img[^>]*>").expect("image tag regex should be valid"));

#[allow(clippy::expect_used)]
static MARKDOWN_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[([^\]]+)\]\([^\)]+\)").expect("markdown link regex should be valid")
});

#[allow(clippy::expect_used)]
static LIST_ITEM_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<li[^>]*>").expect("list item regex should be valid"));

#[allow(clippy::expect_used)]
static INNER_TAG_CONTENT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<(?:style|svg|script|figure)[^>]*>.*?</(?:style|svg|script|figure)>")
        .expect("inner tag content regex should be valid")
});

#[allow(clippy::expect_used)]
static SVG_TAG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<svg[^>]*>").expect("svg tag regex should be valid"));

#[allow(clippy::expect_used)]
static FIGURE_TAG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<figure[^>]*>").expect("figure tag regex should be valid"));

#[allow(clippy::expect_used)]
static SLIDE_COUNTER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d+\.(?:\d+\s+|\s+)").expect("slide counter regex should be valid")
});

#[derive(Debug, Clone, Default)]
pub struct PresentationStats {
    pub total_slides: usize,
    pub total_parts: usize,
    pub slides_per_part: HashMap<String, usize>,
    pub total_words: usize,
    pub words_per_part: HashMap<String, usize>,
    pub total_bullets: usize,
    pub bullets_per_part: HashMap<String, usize>,
    pub total_images: usize,
    pub images_per_part: HashMap<String, usize>,
    pub wpm: u16,
    pub include_notes_in_duration: bool,
    pub total_notes_words: usize,
    pub notes_words_per_part: HashMap<String, usize>,
    pub part_order: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DurationEstimate {
    pub slow: Duration,
    pub normal: Duration,
    pub fast: Duration,
    pub custom: Duration,
    pub image_time: Duration,
}

impl PresentationStats {
    #[must_use]
    pub fn new(wpm: u16, include_notes_in_duration: bool) -> Self {
        Self {
            wpm,
            include_notes_in_duration,
            ..Default::default()
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
                    stats.total_parts += 1;
                    let part_name = slide.title.to_string();
                    current_part = Some(part_name.clone());
                    // Track part order
                    stats.part_order.push(part_name.clone());
                    // Initialize counters for this part
                    stats.slides_per_part.insert(part_name.clone(), 0);
                    stats.words_per_part.insert(part_name.clone(), 0);
                    stats.bullets_per_part.insert(part_name.clone(), 0);
                    stats.images_per_part.insert(part_name.clone(), 0);
                    stats.notes_words_per_part.insert(part_name, 0);
                } else {
                    stats.total_slides += 1;

                    // Analyze slide content (title, body, and notes)
                    // Strip slide counter from title (e.g., "3.5 Diagram" -> "Diagram")
                    let title_stats = analyze_content_strip_counter(&slide.title);
                    let body_stats = analyze_content(&slide.body);
                    let notes_stats = analyze_content(&slide.notes);

                    // Content stats exclude notes to avoid double-counting in duration calculation
                    let content_stats = ContentAnalysis {
                        words: title_stats.words + body_stats.words,
                        bullets: title_stats.bullets + body_stats.bullets + notes_stats.bullets,
                        images: title_stats.images + body_stats.images + notes_stats.images,
                    };

                    stats.total_words += content_stats.words;
                    stats.total_bullets += content_stats.bullets;
                    stats.total_images += content_stats.images;
                    stats.total_notes_words += notes_stats.words;

                    // Add to current part if we're in one, otherwise create an implicit "Introduction" part
                    if let Some(ref part_name) = current_part {
                        *stats.slides_per_part.get_mut(part_name).unwrap_or(&mut 0) += 1;
                        *stats.words_per_part.get_mut(part_name).unwrap_or(&mut 0) +=
                            content_stats.words;
                        *stats.bullets_per_part.get_mut(part_name).unwrap_or(&mut 0) +=
                            content_stats.bullets;
                        *stats.images_per_part.get_mut(part_name).unwrap_or(&mut 0) +=
                            content_stats.images;
                        *stats
                            .notes_words_per_part
                            .get_mut(part_name)
                            .unwrap_or(&mut 0) += notes_stats.words;
                    } else {
                        // Slide before first part - create implicit "Introduction" part
                        if !has_slides_before_first_part {
                            has_slides_before_first_part = true;
                            let intro_part = "(Introduction)".to_string();
                            stats.part_order.insert(0, intro_part.clone());
                            stats.slides_per_part.insert(intro_part.clone(), 0);
                            stats.words_per_part.insert(intro_part.clone(), 0);
                            stats.bullets_per_part.insert(intro_part.clone(), 0);
                            stats.images_per_part.insert(intro_part.clone(), 0);
                            stats.notes_words_per_part.insert(intro_part.clone(), 0);
                            current_part = Some(intro_part);
                        }

                        if let Some(ref part_name) = current_part {
                            *stats.slides_per_part.get_mut(part_name).unwrap_or(&mut 0) += 1;
                            *stats.words_per_part.get_mut(part_name).unwrap_or(&mut 0) +=
                                content_stats.words;
                            *stats.bullets_per_part.get_mut(part_name).unwrap_or(&mut 0) +=
                                content_stats.bullets;
                            *stats.images_per_part.get_mut(part_name).unwrap_or(&mut 0) +=
                                content_stats.images;
                            *stats
                                .notes_words_per_part
                                .get_mut(part_name)
                                .unwrap_or(&mut 0) += notes_stats.words;
                        }
                    }
                }
            }
        }

        stats
    }

    #[must_use]
    pub fn duration_estimates(&self) -> DurationEstimate {
        #[allow(clippy::cast_precision_loss)]
        let base_words = self.total_words as f64;

        #[allow(clippy::cast_precision_loss)]
        let notes_words = if self.include_notes_in_duration {
            self.total_notes_words as f64
        } else {
            0.0
        };

        let total_words = base_words + notes_words;

        DurationEstimate {
            slow: Duration::from_secs_f64((total_words / 110.0) * 60.0),
            normal: Duration::from_secs_f64((total_words / 150.0) * 60.0),
            fast: Duration::from_secs_f64((total_words / 170.0) * 60.0),
            custom: Duration::from_secs_f64((total_words / f64::from(self.wpm)) * 60.0),
            image_time: Duration::from_secs(self.total_images as u64 * 5),
        }
    }

    fn calculate_adjusted_percentages(&self) -> HashMap<String, f64> {
        let mut result = HashMap::new();

        // Calculate effective total including notes if they're included in duration
        let effective_total: usize = if self.include_notes_in_duration {
            self.words_per_part.values().sum::<usize>()
                + self.notes_words_per_part.values().sum::<usize>()
        } else {
            self.words_per_part.values().sum::<usize>()
        };

        if effective_total == 0 {
            return result;
        }

        // Calculate raw percentages and round to 1 decimal place
        // Keep original part order from slides_per_part
        let parts_data = self
            .slides_per_part
            .keys()
            .map(|name| {
                let words = self.words_per_part.get(name).unwrap_or(&0);
                let part_total = if self.include_notes_in_duration {
                    words + self.notes_words_per_part.get(name).unwrap_or(&0)
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
        if self.total_parts > 0 {
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
        duration: &DurationEstimate,
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
                self.total_slides.to_string().cyan()
            )?;
            if self.total_parts > 0 {
                writeln!(writer, "  • Parts: {}", self.total_parts.to_string().cyan())?;
            }
            writeln!(writer, "  • Words: {}", self.total_words.to_string().cyan())?;
            if self.total_notes_words > 0 {
                writeln!(
                    writer,
                    "  • Notes words: {}{}",
                    self.total_notes_words.to_string().cyan(),
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
                self.total_bullets.to_string().cyan()
            )?;
            writeln!(
                writer,
                "  • Images: {}",
                self.total_images.to_string().cyan()
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
            writeln!(writer, "  • Total slides: {}", self.total_slides)?;
            if self.total_parts > 0 {
                writeln!(writer, "  • Parts: {}", self.total_parts)?;
            }
            writeln!(writer, "  • Words: {}", self.total_words)?;
            if self.total_notes_words > 0 {
                writeln!(
                    writer,
                    "  • Notes words: {} ({})",
                    self.total_notes_words,
                    if self.include_notes_in_duration {
                        "included in duration"
                    } else {
                        "not included in duration"
                    }
                )?;
            }
            writeln!(writer, "  • Bullet points: {}", self.total_bullets)?;
            writeln!(writer, "  • Images: {}", self.total_images)?;
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
        for part_name in &self.part_order {
            let slide_count = self.slides_per_part.get(part_name).unwrap_or(&0);
            let words = self.words_per_part.get(part_name).unwrap_or(&0);
            let notes_words = if self.include_notes_in_duration {
                self.notes_words_per_part.get(part_name).unwrap_or(&0)
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
        duration: &DurationEstimate,
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
        if self.total_images > 0 {
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
        if self.total_parts > 1 {
            let mut max_percentage = 0.0;
            let mut max_part = String::new();

            for (part_name, &words) in &self.words_per_part {
                #[allow(clippy::cast_precision_loss)]
                let percentage = (words as f64 / self.total_words as f64) * 100.0;
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
        if self.total_slides > 0 {
            let avg_words_per_slide = self.total_words / self.total_slides;
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

#[derive(Debug, Clone, Default)]
struct ContentAnalysis {
    words: usize,
    bullets: usize,
    images: usize,
}

fn analyze_content(content: &Content) -> ContentAnalysis {
    analyze_content_internal(content, false)
}

fn analyze_content_strip_counter(content: &Content) -> ContentAnalysis {
    analyze_content_internal(content, true)
}

fn analyze_content_internal(content: &Content, strip_counter: bool) -> ContentAnalysis {
    let mut analysis = ContentAnalysis::default();

    match content {
        Content::Empty => {
            // No content to analyze
        }
        Content::Text { text } => {
            let text_to_analyze = if strip_counter {
                strip_slide_counter(text)
            } else {
                text.clone()
            };
            analysis.words += count_words(&text_to_analyze);
            analysis.bullets += count_bullet_points(&text_to_analyze);
        }
        Content::Html { raw, alt, style: _ } => {
            // First remove style, svg, and script tags with their content (shouldn't count as words)
            let cleaned_html = remove_inner_tag_content(raw);
            // Then strip remaining HTML tags
            let mut text = strip_html_tags(&cleaned_html);
            if strip_counter {
                text = strip_slide_counter(&text);
            }
            analysis.words += count_words(&text);
            // Count bullets from both markdown-style and HTML list items
            analysis.bullets += count_bullet_points(&text);
            analysis.bullets += count_list_items_in_html(raw);
            analysis.images += count_images_in_html(raw);

            // Also count alt text if present
            if let Some(alt_text) = alt {
                analysis.words += count_words(alt_text);
            }
        }
        Content::Grid { cells, style: _ } => {
            // Recursively analyze nested content
            for cell in cells {
                let nested = analyze_content_internal(cell, strip_counter);
                analysis.words += nested.words;
                analysis.bullets += nested.bullets;
                analysis.images += nested.images;
            }
        }
    }

    analysis
}

fn count_words(text: &str) -> usize {
    // Strip markdown links, keeping only the link text
    let text_without_link_urls = MARKDOWN_LINK_REGEX.replace_all(text, "$1");

    text_without_link_urls
        .split_whitespace()
        .filter(|word| !word.trim().is_empty())
        .count()
}

fn count_bullet_points(text: &str) -> usize {
    BULLET_REGEX.find_iter(text).count() + NUMBERED_REGEX.find_iter(text).count()
}

fn strip_html_tags(html: &str) -> String {
    HTML_TAG_REGEX.replace_all(html, " ").to_string()
}

fn count_images_in_html(html: &str) -> usize {
    IMG_TAG_REGEX.find_iter(html).count()
        + SVG_TAG_REGEX.find_iter(html).count()
        + FIGURE_TAG_REGEX.find_iter(html).count()
}

fn count_list_items_in_html(html: &str) -> usize {
    LIST_ITEM_REGEX.find_iter(html).count()
}

fn remove_inner_tag_content(html: &str) -> String {
    INNER_TAG_CONTENT_REGEX.replace_all(html, " ").to_string()
}

fn strip_slide_counter(text: &str) -> String {
    SLIDE_COUNTER_REGEX.replace(text, "").to_string()
}

#[cfg(test)]
mod tests {
    use toboggan_core::{Slide, Style};

    use super::*;
    use crate::{ParseResult, SlideProcessingResult, TalkMetadata};

    #[test]
    fn test_word_counting() {
        assert_eq!(count_words("Hello world"), 2);
        assert_eq!(count_words("  Hello   world  "), 2);
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("One two three four five"), 5);
    }

    #[test]
    fn test_word_counting_with_markdown_links() {
        // Single link - should count only the link text, not the URL
        assert_eq!(count_words("Check [this link](https://example.com)"), 3);

        // Multiple words in link text
        assert_eq!(
            count_words("Visit [my awesome website](https://example.com) today"),
            5
        );

        // Multiple links
        assert_eq!(
            count_words(
                "See [docs](https://docs.example.com) and [source](https://github.com/example)"
            ),
            4
        );

        // Link with complex URL
        assert_eq!(
            count_words(
                "Read [the article](https://example.com/path/to/article?param=value&other=123)"
            ),
            3
        );

        // Text without links should work as before
        assert_eq!(count_words("No links here just text"), 5);

        // Mixed content
        assert_eq!(
            count_words(
                "Start text [link one](https://url1.com) middle [link two](https://url2.com) end"
            ),
            8
        );
    }

    #[test]
    fn test_bullet_point_counting() {
        let text = "
- First item
- Second item
* Third item
+ Fourth item
1. Numbered item
2. Another numbered
";
        assert_eq!(count_bullet_points(text), 6);
    }

    #[test]
    fn test_html_image_counting() {
        // Test img tags
        let html = r#"<p>Some text <img src="image1.jpg" alt="test"> more text <img src="image2.png"></p>"#;
        assert_eq!(count_images_in_html(html), 2);

        // Test svg tags
        let html =
            r#"<p>Text</p><svg width="100" height="100"><circle cx="50" cy="50" r="40"/></svg>"#;
        assert_eq!(count_images_in_html(html), 1, "Should count SVG as image");

        // Test figure tags
        let html = r#"<figure><img src="test.jpg"><figcaption>Caption</figcaption></figure>"#;
        assert_eq!(
            count_images_in_html(html),
            2,
            "Should count both figure and img inside"
        );

        // Test multiple types
        let html = r#"
            <img src="photo.jpg">
            <svg><path d="M0,0"/></svg>
            <figure><img src="chart.png"><figcaption>Chart</figcaption></figure>
            <svg width="50" height="50"><rect/></svg>
        "#;
        assert_eq!(
            count_images_in_html(html),
            5,
            "Should count: 1 img + 2 svg + 1 figure + 1 img inside figure"
        );

        // Test svg with attributes
        let html = r#"<svg class="icon" viewBox="0 0 24 24"><path/></svg>"#;
        assert_eq!(count_images_in_html(html), 1);

        // Test figure without img
        let html = r"<figure><pre>Code block</pre><figcaption>Code</figcaption></figure>";
        assert_eq!(
            count_images_in_html(html),
            1,
            "Figure alone counts as image"
        );
    }

    #[test]
    fn test_html_list_item_counting() {
        // Unordered list
        let html = r"<ul><li>Item 1</li><li>Item 2</li><li>Item 3</li></ul>";
        assert_eq!(count_list_items_in_html(html), 3);

        // Ordered list
        let html = r"<ol><li>First</li><li>Second</li></ol>";
        assert_eq!(count_list_items_in_html(html), 2);

        // Mixed list with attributes
        let html = r#"<ul><li class="active">Item 1</li><li id="item2">Item 2</li></ul>"#;
        assert_eq!(count_list_items_in_html(html), 2);

        // Nested lists
        let html =
            r"<ul><li>Outer 1<ul><li>Inner 1</li><li>Inner 2</li></ul></li><li>Outer 2</li></ul>";
        assert_eq!(count_list_items_in_html(html), 4);
    }

    #[test]
    fn test_analyze_html_content_with_bullets() {
        // Test that HTML list items are counted as bullets
        let html_content = Content::Html {
            raw: r"<ul><li>Item 1</li><li>Item 2</li><li>Item 3</li></ul>".to_string(),
            style: Style::default(),
            alt: None,
        };

        let stats = analyze_content(&html_content);
        assert_eq!(stats.bullets, 3, "Should count 3 list items as bullets");
        assert_eq!(
            stats.words, 6,
            "Should count 6 words (Item 1, Item 2, Item 3)"
        );
    }

    #[test]
    fn test_remove_inner_tag_content() {
        // Test style tag removal
        let html = r"<p>Some text</p><style>body { color: red; }</style><p>More text</p>";
        let cleaned = remove_inner_tag_content(html);
        assert!(!cleaned.contains("color"));
        assert!(cleaned.contains("Some text"));
        assert!(cleaned.contains("More text"));

        // Test svg tag removal
        let html = r#"<div>Text</div><svg><path d="M0,0 L10,10"/></svg><div>More</div>"#;
        let cleaned = remove_inner_tag_content(html);
        assert!(!cleaned.contains("path"));
        assert!(!cleaned.contains("M0,0"));
        assert!(cleaned.contains("Text"));
        assert!(cleaned.contains("More"));

        // Test script tag removal
        let html = r#"<p>Content</p><script>console.log("test");</script><p>End</p>"#;
        let cleaned = remove_inner_tag_content(html);
        assert!(!cleaned.contains("console"));
        assert!(!cleaned.contains("log"));
        assert!(cleaned.contains("Content"));
        assert!(cleaned.contains("End"));

        // Test case insensitivity
        let html = r"<p>Text</p><STYLE>body{}</STYLE><p>More</p>";
        let cleaned = remove_inner_tag_content(html);
        assert!(!cleaned.contains("body{}"));

        // Test figure tag removal
        let html = r#"<p>Text</p><figure><img src="test.jpg"><figcaption>Caption text</figcaption></figure><p>More</p>"#;
        let cleaned = remove_inner_tag_content(html);
        assert!(!cleaned.contains("Caption text"));
        assert!(!cleaned.contains("figcaption"));
        assert!(cleaned.contains("Text"));
        assert!(cleaned.contains("More"));

        // Test multiple tags
        let html = r"<p>A</p><style>a{}</style><p>B</p><svg>x</svg><p>C</p><script>y</script><p>D</p><figure>z</figure><p>E</p>";
        let cleaned = remove_inner_tag_content(html);
        assert!(!cleaned.contains("a{}"));
        assert!(!cleaned.contains(">x<"));
        assert!(!cleaned.contains(">y<"));
        assert!(!cleaned.contains(">z<"));
        assert!(cleaned.contains('A'));
        assert!(cleaned.contains('B'));
        assert!(cleaned.contains('C'));
        assert!(cleaned.contains('D'));
        assert!(cleaned.contains('E'));
    }

    #[test]
    fn test_word_counting_excludes_inner_tags() {
        // Style tag content should not be counted
        let html_content = Content::Html {
            raw: r"<p>Hello world</p><style>.class { color: blue; font-size: 12px; }</style>"
                .to_string(),
            style: Style::default(),
            alt: None,
        };
        let stats = analyze_content(&html_content);
        assert_eq!(
            stats.words, 2,
            "Should only count 'Hello world', not CSS properties"
        );

        // SVG content should not be counted
        let html_content = Content::Html {
            raw: r#"<p>One two three</p><svg><path d="M0,0 L10,10 L20,20"/><circle cx="5" cy="5" r="3"/></svg>"#.to_string(),
            style: Style::default(),
            alt: None,
        };
        let stats = analyze_content(&html_content);
        assert_eq!(stats.words, 3, "Should only count 'One two three', not SVG");

        // Script content should not be counted
        let html_content = Content::Html {
            raw: r"<p>Test content</p><script>const x = 10; const y = 20; console.log(x + y);</script>".to_string(),
            style: Style::default(),
            alt: None,
        };
        let stats = analyze_content(&html_content);
        assert_eq!(
            stats.words, 2,
            "Should only count 'Test content', not JavaScript"
        );

        // Figure content (including figcaption) should not be counted as words
        let html_content = Content::Html {
            raw: r#"<p>Main text here</p><figure><img src="chart.png"><figcaption>This is a long caption with many words</figcaption></figure>"#.to_string(),
            style: Style::default(),
            alt: None,
        };
        let stats = analyze_content(&html_content);
        assert_eq!(
            stats.words, 3,
            "Should only count 'Main text here', not figcaption content"
        );
        assert_eq!(stats.images, 2, "Should count both figure and img");
    }

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

        assert_eq!(stats.total_slides, 1); // Only non-part slides
        assert_eq!(stats.total_parts, 1);
        assert!(stats.total_words > 0);
        // Verify intro slide is counted in the breakdown
        let total_in_parts: usize = stats.slides_per_part.values().sum();
        assert_eq!(
            total_in_parts, stats.total_slides,
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

        assert_eq!(stats.total_slides, 1, "Should have 1 slide");
        assert_eq!(stats.total_images, 1, "Should count SVG as 1 image");
        // Counter "3.5 " is stripped, only "Diagram" counted
        assert_eq!(
            stats.total_words, 1,
            "Title '3.5 Diagram' counts as 1 word (counter stripped, text inside SVG excluded)"
        );
    }

    #[test]
    fn test_strip_slide_counter() {
        assert_eq!(strip_slide_counter("3.5 Diagram"), "Diagram");
        assert_eq!(strip_slide_counter("1. Introduction"), "Introduction");
        assert_eq!(strip_slide_counter("Diagram"), "Diagram");
        assert_eq!(strip_slide_counter("10.20 Test"), "Test");
        assert_eq!(strip_slide_counter(""), "");
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
            stats.total_words, 8,
            "total_words should count title + body (2 + 6 = 8)"
        );
        assert_eq!(
            stats.total_notes_words, 4,
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
}
