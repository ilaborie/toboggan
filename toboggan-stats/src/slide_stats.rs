use std::collections::HashMap;
use std::time::Duration;

use toboggan_core::{Content, Slide, SlideKind, Talk};

use crate::analysis::{
    count_images_in_html, count_list_items_in_html, count_steps_from_content, count_words,
    extract_text_from_html, strip_slide_counter,
};

/// Statistics for a single slide
#[derive(Debug, Clone, Default)]
pub struct SlideStats {
    /// Number of words in title and body (excluding notes)
    pub words: usize,
    /// Number of bullet points (`<li>` elements)
    pub bullets: usize,
    /// Number of images (`img`, `svg`, `figure` elements)
    pub images: usize,
    /// Number of animation steps (`.step` CSS class elements)
    pub steps: usize,
    /// Number of words in speaker notes
    pub notes_words: usize,
}

impl SlideStats {
    /// Compute statistics for a slide
    #[must_use]
    pub fn from_slide(slide: &Slide) -> Self {
        Self::from_slide_with_options(slide, false)
    }

    /// Compute statistics for a slide, optionally stripping slide counter from title
    #[must_use]
    pub fn from_slide_with_options(slide: &Slide, strip_counter: bool) -> Self {
        let title_analysis = analyze_content(&slide.title, strip_counter);
        let body_analysis = analyze_content(&slide.body, false);
        let notes_analysis = analyze_content(&slide.notes, false);

        // Count steps from body content
        let steps = count_steps_from_content(&slide.body);

        Self {
            words: title_analysis.words + body_analysis.words,
            bullets: title_analysis.bullets + body_analysis.bullets + notes_analysis.bullets,
            images: title_analysis.images + body_analysis.images + notes_analysis.images,
            steps,
            notes_words: notes_analysis.words,
        }
    }
}

/// Duration estimates at different speaking rates
#[derive(Debug, Clone)]
pub struct DurationEstimate {
    /// Duration at 110 WPM (slow speaker)
    pub slow: Duration,
    /// Duration at 150 WPM (normal speaker)
    pub normal: Duration,
    /// Duration at 170 WPM (fast speaker)
    pub fast: Duration,
    /// Duration at custom WPM
    pub custom: Duration,
    /// Additional time for viewing images (5 seconds each)
    pub image_time: Duration,
}

/// Aggregate statistics for a presentation
#[derive(Debug, Clone, Default)]
pub struct PresentationStats {
    /// Total number of slides (excluding Part slides)
    pub total_slides: usize,
    /// Total number of parts
    pub total_parts: usize,
    /// Slides per part
    pub slides_per_part: HashMap<String, usize>,
    /// Total word count (title + body, excluding notes)
    pub total_words: usize,
    /// Words per part
    pub words_per_part: HashMap<String, usize>,
    /// Total bullet points
    pub total_bullets: usize,
    /// Bullets per part
    pub bullets_per_part: HashMap<String, usize>,
    /// Total images
    pub total_images: usize,
    /// Images per part
    pub images_per_part: HashMap<String, usize>,
    /// Total animation steps
    pub total_steps: usize,
    /// Steps per part
    pub steps_per_part: HashMap<String, usize>,
    /// Total words in speaker notes
    pub total_notes_words: usize,
    /// Notes words per part
    pub notes_words_per_part: HashMap<String, usize>,
    /// Part names in order of appearance
    pub part_order: Vec<String>,
}

impl PresentationStats {
    /// Compute statistics from a Talk
    #[must_use]
    pub fn from_talk(talk: &Talk) -> Self {
        Self::from_slides(&talk.slides)
    }

    /// Compute statistics from a slice of slides
    #[must_use]
    pub fn from_slides(slides: &[Slide]) -> Self {
        let mut stats = Self::default();
        let mut current_part: Option<String> = None;
        let mut has_slides_before_first_part = false;

        for slide in slides {
            if slide.kind == SlideKind::Part {
                stats.total_parts += 1;
                let part_name = slide.title.to_string();
                current_part = Some(part_name.clone());
                stats.part_order.push(part_name.clone());
                stats.slides_per_part.insert(part_name.clone(), 0);
                stats.words_per_part.insert(part_name.clone(), 0);
                stats.bullets_per_part.insert(part_name.clone(), 0);
                stats.images_per_part.insert(part_name.clone(), 0);
                stats.steps_per_part.insert(part_name.clone(), 0);
                stats.notes_words_per_part.insert(part_name, 0);
            } else {
                stats.total_slides += 1;

                // Compute slide stats with counter stripping for title
                let slide_stats = SlideStats::from_slide_with_options(slide, true);

                stats.total_words += slide_stats.words;
                stats.total_bullets += slide_stats.bullets;
                stats.total_images += slide_stats.images;
                stats.total_steps += slide_stats.steps;
                stats.total_notes_words += slide_stats.notes_words;

                // Add to current part or create implicit "Introduction"
                if current_part.is_none() && !has_slides_before_first_part {
                    has_slides_before_first_part = true;
                    let intro_part = "(Introduction)".to_string();
                    stats.part_order.insert(0, intro_part.clone());
                    stats.slides_per_part.insert(intro_part.clone(), 0);
                    stats.words_per_part.insert(intro_part.clone(), 0);
                    stats.bullets_per_part.insert(intro_part.clone(), 0);
                    stats.images_per_part.insert(intro_part.clone(), 0);
                    stats.steps_per_part.insert(intro_part.clone(), 0);
                    stats.notes_words_per_part.insert(intro_part.clone(), 0);
                    current_part = Some(intro_part);
                }

                if let Some(ref part_name) = current_part {
                    *stats.slides_per_part.get_mut(part_name).unwrap_or(&mut 0) += 1;
                    *stats.words_per_part.get_mut(part_name).unwrap_or(&mut 0) += slide_stats.words;
                    *stats.bullets_per_part.get_mut(part_name).unwrap_or(&mut 0) +=
                        slide_stats.bullets;
                    *stats.images_per_part.get_mut(part_name).unwrap_or(&mut 0) +=
                        slide_stats.images;
                    *stats.steps_per_part.get_mut(part_name).unwrap_or(&mut 0) += slide_stats.steps;
                    *stats
                        .notes_words_per_part
                        .get_mut(part_name)
                        .unwrap_or(&mut 0) += slide_stats.notes_words;
                }
            }
        }

        stats
    }

    /// Calculate duration estimates at different speaking rates
    #[must_use]
    pub fn duration_estimates(
        &self,
        wpm: u16,
        include_notes_in_duration: bool,
    ) -> DurationEstimate {
        #[allow(clippy::cast_precision_loss)]
        let base_words = self.total_words as f64;

        #[allow(clippy::cast_precision_loss)]
        let notes_words = if include_notes_in_duration {
            self.total_notes_words as f64
        } else {
            0.0
        };

        let total_words = base_words + notes_words;

        DurationEstimate {
            slow: Duration::from_secs_f64((total_words / 110.0) * 60.0),
            normal: Duration::from_secs_f64((total_words / 150.0) * 60.0),
            fast: Duration::from_secs_f64((total_words / 170.0) * 60.0),
            custom: Duration::from_secs_f64((total_words / f64::from(wpm)) * 60.0),
            image_time: Duration::from_secs(self.total_images as u64 * 5),
        }
    }
}

/// Internal content analysis result
#[derive(Debug, Clone, Default)]
struct ContentAnalysis {
    words: usize,
    bullets: usize,
    images: usize,
}

fn analyze_content(content: &Content, strip_counter: bool) -> ContentAnalysis {
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
            analysis.words = count_words(&text_to_analyze);
        }
        Content::Html { raw, alt, style: _ } => {
            let mut text = extract_text_from_html(raw);
            if strip_counter {
                text = strip_slide_counter(&text);
            }
            analysis.words = count_words(&text);
            analysis.bullets = count_list_items_in_html(raw);
            analysis.images = count_images_in_html(raw);

            // Also count alt text if present
            if let Some(alt_text) = alt {
                analysis.words += count_words(alt_text);
            }
        }
        Content::Grid { cells, style: _ } => {
            for cell in cells {
                let nested = analyze_content(cell, strip_counter);
                analysis.words += nested.words;
                analysis.bullets += nested.bullets;
                analysis.images += nested.images;
            }
        }
    }

    analysis
}

#[cfg(test)]
mod tests {
    use toboggan_core::Style;

    use super::*;

    #[test]
    fn test_slide_stats_from_slide() {
        let slide = Slide::new("Test Slide").with_body(Content::Text {
            text: "Hello world test".to_string(),
        });

        let stats = SlideStats::from_slide(&slide);
        assert_eq!(stats.words, 2 + 3); // "Test Slide" + "Hello world test"
        assert_eq!(stats.bullets, 0);
        assert_eq!(stats.images, 0);
        assert_eq!(stats.steps, 0);
        assert_eq!(stats.notes_words, 0);
    }

    #[test]
    fn test_slide_stats_with_html() {
        let slide = Slide::new("Title").with_body(Content::Html {
            raw: r"<ul><li>Item 1</li><li>Item 2</li></ul><img src='test.jpg'>".to_string(),
            style: Style::default(),
            alt: None,
        });

        let stats = SlideStats::from_slide(&slide);
        assert_eq!(stats.bullets, 2);
        assert_eq!(stats.images, 1);
    }

    #[test]
    fn test_slide_stats_with_steps() {
        let slide = Slide::new("Title").with_body(Content::Html {
            raw: r#"<div class="step">Step 1</div><div class="step">Step 2</div>"#.to_string(),
            style: Style::default(),
            alt: None,
        });

        let stats = SlideStats::from_slide(&slide);
        assert_eq!(stats.steps, 2);
    }

    #[test]
    fn test_slide_stats_with_notes() {
        let slide = Slide::new("Title")
            .with_body(Content::Text {
                text: "Body content".to_string(),
            })
            .with_notes(Content::Text {
                text: "Speaker notes here".to_string(),
            });

        let stats = SlideStats::from_slide(&slide);
        assert_eq!(stats.words, 1 + 2); // "Title" + "Body content"
        assert_eq!(stats.notes_words, 3); // "Speaker notes here"
    }

    #[test]
    fn test_presentation_stats_from_slides() {
        let slides = vec![
            Slide::new("Introduction").with_body(Content::Text {
                text: "Welcome everyone".to_string(),
            }),
            Slide::part("Part One"),
            Slide::new("Content").with_body(Content::Text {
                text: "Some content here".to_string(),
            }),
        ];

        let stats = PresentationStats::from_slides(&slides);

        assert_eq!(stats.total_slides, 2); // Intro + Content (Part not counted)
        assert_eq!(stats.total_parts, 1);
        assert_eq!(stats.part_order.len(), 2); // (Introduction) + Part One
    }

    #[test]
    fn test_presentation_stats_duration_estimates() {
        let slides = vec![
            Slide::new("Title").with_body(Content::Text {
                // 149 words in body + 1 word in title = 150 words at 150 WPM = 1 minute
                text: (0..149)
                    .map(|idx| format!("word{idx}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            }),
        ];

        let stats = PresentationStats::from_slides(&slides);
        let duration = stats.duration_estimates(150, false);

        // 150 words / 150 WPM = 1 minute = 60 seconds
        assert_eq!(duration.custom.as_secs(), 60);
    }

    #[test]
    fn test_presentation_stats_with_notes_in_duration() {
        let slides = vec![
            Slide::new("Title")
                .with_body(Content::Text {
                    text: "one two three four".to_string(),
                })
                .with_notes(Content::Text {
                    text: "note1 note2".to_string(),
                }),
        ];

        let stats = PresentationStats::from_slides(&slides);

        // Without notes: 5 words (Title + one two three four)
        let duration_no_notes = stats.duration_estimates(150, false);

        // With notes: 5 + 2 = 7 words
        let duration_with_notes = stats.duration_estimates(150, true);

        assert!(duration_with_notes.custom > duration_no_notes.custom);
    }
}
