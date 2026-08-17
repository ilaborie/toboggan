use toboggan_core::SlideKind;

use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{Rule, RuleContext};

/// A content slide with too many words.
pub(crate) struct ExcessiveWords;

impl Rule for ExcessiveWords {
    fn id(&self) -> RuleId {
        super::ids::CONTENT_EXCESSIVE_WORDS
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        if context.slide.kind != SlideKind::Standard {
            return;
        }
        let words = context.stats().words;
        let max = context.config.max_words_per_slide;
        if words > max {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    format!("{words} words (suggested limit {max})"),
                )
                .with_help("split dense slides; prefer fewer words per slide"),
            );
        }
    }
}

/// A slide with too many images.
pub(crate) struct TooManyImages;

impl Rule for TooManyImages {
    fn id(&self) -> RuleId {
        super::ids::CONTENT_TOO_MANY_IMAGES
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let images = context.stats().images;
        let max = context.config.max_images_per_slide;
        if images > max {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    format!("{images} images (suggested limit {max})"),
                )
                .with_help("consider splitting the slide or using a gallery"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use toboggan_core::{Content, Slide};

    use super::*;
    use crate::rule::LintConfig;
    use crate::rules::test_support::slide_diagnostics_with;

    const IMG: &str = r#"<img src="a.png" alt="a">"#;

    fn limits() -> LintConfig {
        LintConfig {
            max_words_per_slide: 3,
            max_images_per_slide: 1,
            ..LintConfig::default()
        }
    }

    /// A part slide is a section title; counting its handful of words against
    /// a content-slide budget would report every deck's own structure.
    #[test]
    fn only_content_slides_are_judged_on_word_count() {
        let wordy = "<p>one two three four five</p>";
        let standard = Slide::new("T").with_body(Content::html(wordy));
        assert_eq!(
            slide_diagnostics_with(&ExcessiveWords, &standard, &limits()).len(),
            1
        );

        for exempt in [Slide::cover("T"), Slide::part("T")] {
            let slide = exempt.with_body(Content::html(wordy));
            assert!(
                slide_diagnostics_with(&ExcessiveWords, &slide, &limits()).is_empty(),
                "{:?} is not a content slide",
                slide.kind
            );
        }
    }

    /// Deliberately unlike the word rule: a cover crowded with images is
    /// still crowded, so this one applies to every kind.
    #[test]
    fn the_image_limit_applies_to_every_kind() {
        for slide in [Slide::new("T"), Slide::cover("T"), Slide::part("T")] {
            let kind = slide.kind;
            let slide = slide.with_body(Content::html(format!("{IMG}{IMG}")));
            assert_eq!(
                slide_diagnostics_with(&TooManyImages, &slide, &limits()).len(),
                1,
                "{kind:?} should be judged too"
            );
        }
    }

    /// Both thresholds are `>`, so sitting exactly on the limit is quiet.
    #[test]
    fn the_limits_are_exclusive() {
        let at_word_limit = Slide::default().with_body(Content::html("<p>one two three</p>"));
        assert!(slide_diagnostics_with(&ExcessiveWords, &at_word_limit, &limits()).is_empty());

        let at_image_limit = Slide::new("T").with_body(Content::html(IMG));
        assert!(slide_diagnostics_with(&TooManyImages, &at_image_limit, &limits()).is_empty());
    }
}
