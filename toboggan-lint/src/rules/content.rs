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
