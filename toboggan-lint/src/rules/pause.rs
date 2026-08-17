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

#[cfg(test)]
mod tests {
    use toboggan_core::{Content, Slide};

    use super::*;
    use crate::rules::test_support::{fires, only, slide_diagnostics};

    const STEP: &str = r#"<div class="step">reveal</div>"#;

    fn with_body(mut slide: Slide, body: &str) -> Slide {
        slide.body = Content::html(body);
        slide
    }

    /// A cover or a part is on screen as a whole; revealing it in pieces has
    /// nothing to reveal. The message names the kind, so both are checked.
    #[test]
    fn a_step_on_a_cover_or_part_is_reported() {
        for (slide, name) in [
            (Slide::cover("Deck"), "cover"),
            (Slide::part("Section"), "part"),
        ] {
            let diagnostics = slide_diagnostics(&PauseInPart, &with_body(slide, STEP));
            let message = &only(&diagnostics).message;
            assert!(message.starts_with(name), "{message}");
        }
    }

    #[test]
    fn steps_on_a_content_slide_are_the_point() {
        assert!(!fires(&PauseInPart, &with_body(Slide::new("T"), STEP)));
    }

    /// A stray `<!-- pause -->` renders a step with nothing in it: the deck
    /// gains a keypress that changes nothing on screen.
    #[test]
    fn a_step_revealing_nothing_is_reported() {
        let slide = with_body(Slide::new("T"), r#"<div class="step"></div>"#);
        assert!(fires(&EmptyStep, &slide));
    }

    /// An element with no text still reveals something — an image, a diagram —
    /// so whitespace-only is the boundary, not "no text".
    #[test]
    fn a_step_holding_only_an_element_is_not_empty() {
        let slide = with_body(
            Slide::new("T"),
            r#"<div class="step"><img src="a.png" alt="a"></div>"#,
        );
        assert!(!fires(&EmptyStep, &slide));
    }
}
