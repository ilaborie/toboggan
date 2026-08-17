use toboggan_core::SlideKind;

use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{Rule, RuleContext};

/// Cover and part (section) slides should not contain reveal steps.
pub(crate) struct PauseInPart;

impl Rule for PauseInPart {
    fn id(&self) -> RuleId {
        super::ids::PAUSE_IN_PART
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let kind = match context.slide.kind {
            SlideKind::Cover => "cover",
            SlideKind::Part => "part",
            SlideKind::Standard => return,
        };
        let steps = context.body_doc().count_steps();
        if steps > 0 {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    format!("{kind} slide has {steps} reveal step(s)"),
                )
                .with_help("remove `pause` comments from cover and part (section) slides"),
            );
        }
    }
}

/// Steps that reveal nothing (empty `<!-- pause -->`).
pub(crate) struct EmptyStep;

impl Rule for EmptyStep {
    fn id(&self) -> RuleId {
        super::ids::PAUSE_EMPTY_STEP
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let empty = context.body_doc().count_empty_steps();
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
        super::ids::PAUSE_TOO_MANY_STEPS
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let steps = context.body_doc().count_steps();
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
