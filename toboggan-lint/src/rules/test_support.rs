//! Helpers for testing one rule in isolation.
//!
//! The suite in `lib.rs` drives every rule at once through [`crate::lint`],
//! which is the right shape for testing the runner — disable directives,
//! severity overrides, path stamping — and the wrong shape for testing a rule:
//! a finding has to be picked out of everything else the deck triggered, and a
//! rule that never fires looks the same as one whose diagnostic was filtered.
//! These run a single rule and hand back exactly what it said.

use toboggan_core::{Slide, Talk};

use crate::diagnostic::{LintDiagnostic, SlideRef};
use crate::rule::{LintConfig, Rule, RuleContext};

/// Runs `rule` over `slide` with the default configuration.
pub(crate) fn slide_diagnostics(rule: &dyn Rule, slide: &Slide) -> Vec<LintDiagnostic> {
    slide_diagnostics_with(rule, slide, &LintConfig::default())
}

/// Runs `rule` over `slide` with an explicit configuration, for threshold rules.
pub(crate) fn slide_diagnostics_with(
    rule: &dyn Rule,
    slide: &Slide,
    config: &LintConfig,
) -> Vec<LintDiagnostic> {
    let talk = talk_of(std::slice::from_ref(slide));
    let slide_ref = SlideRef::new(0, slide);
    let context = RuleContext::new(&talk, slide, &slide_ref, config);
    let mut out = Vec::new();
    rule.check_slide(&context, &mut out);
    out
}

/// Runs a talk-level `rule` over `talk`.
pub(crate) fn talk_diagnostics(rule: &dyn Rule, talk: &Talk) -> Vec<LintDiagnostic> {
    talk_diagnostics_with(rule, talk, &LintConfig::default())
}

/// Runs a talk-level `rule` over `talk` with an explicit configuration.
pub(crate) fn talk_diagnostics_with(
    rule: &dyn Rule,
    talk: &Talk,
    config: &LintConfig,
) -> Vec<LintDiagnostic> {
    let mut out = Vec::new();
    rule.check_talk(talk, config, &mut out);
    out
}

/// Whether `rule` reports anything about `slide`.
pub(crate) fn fires(rule: &dyn Rule, slide: &Slide) -> bool {
    !slide_diagnostics(rule, slide).is_empty()
}

/// The one diagnostic in `diagnostics`, failing the test if there is not
/// exactly one. Reads better than indexing, and says what went wrong.
pub(crate) fn only(diagnostics: &[LintDiagnostic]) -> &LintDiagnostic {
    match diagnostics {
        [one] => one,
        other => panic!("expected exactly one diagnostic, got {other:#?}"),
    }
}

/// A talk holding `slides`, for rules that need one.
pub(crate) fn talk_of(slides: &[Slide]) -> Talk {
    let mut talk = Talk::new("Test");
    talk.slides = slides.to_vec();
    talk
}
