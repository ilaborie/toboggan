use toboggan_core::SlideKind;
use toboggan_stats::SlideStats;

use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{Rule, RuleContext};

/// A content slide with too many words.
pub(crate) struct ExcessiveWords;

impl Rule for ExcessiveWords {
    fn id(&self) -> RuleId {
        RuleId("content/excessive-words")
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        if context.slide.kind != SlideKind::Standard {
            return;
        }
        let words = SlideStats::from_slide(context.slide).words;
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
        RuleId("content/too-many-images")
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let images = SlideStats::from_slide(context.slide).images;
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
