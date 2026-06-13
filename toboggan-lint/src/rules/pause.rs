use toboggan_core::SlideKind;
use toboggan_stats::HtmlDocument;

use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{Rule, RuleContext, body_html};

/// Part (section) slides should not contain reveal steps.
pub(crate) struct PauseInPart;

impl Rule for PauseInPart {
    fn id(&self) -> RuleId {
        RuleId("pause/in-part")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        if context.slide.kind != SlideKind::Part {
            return;
        }
        let steps = HtmlDocument::parse_fragment(body_html(context.slide)).count_steps();
        if steps > 0 {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    format!("part slide has {steps} reveal step(s)"),
                )
                .with_help("remove `pause` comments from part (section) slides"),
            );
        }
    }
}

/// Steps that reveal nothing (empty `<!-- pause -->`).
pub(crate) struct EmptyStep;

impl Rule for EmptyStep {
    fn id(&self) -> RuleId {
        RuleId("pause/empty-step")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let empty = HtmlDocument::parse_fragment(body_html(context.slide)).count_empty_steps();
        if empty > 0 {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    format!("{empty} empty reveal step(s)"),
                )
                .with_help("remove stray `pause` comments that reveal no content"),
            );
        }
    }
}

/// Slides with an excessive number of reveal steps.
pub(crate) struct TooManySteps;

impl Rule for TooManySteps {
    fn id(&self) -> RuleId {
        RuleId("pause/too-many-steps")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let steps = HtmlDocument::parse_fragment(body_html(context.slide)).count_steps();
        let max = context.config.max_steps_per_slide;
        if steps > max {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    format!("{steps} reveal steps (limit {max})"),
                )
                .with_help("split this slide or reduce the number of `pause` comments"),
            );
        }
    }
}
