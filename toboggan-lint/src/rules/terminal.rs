use std::collections::HashSet;

use toboggan_core::SlideKind;

use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{Rule, RuleContext};

/// Part (section) slides should not declare embedded terminals.
pub(crate) struct TerminalInPart;

impl Rule for TerminalInPart {
    fn id(&self) -> RuleId {
        super::ids::TERM_IN_PART
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        if context.slide.kind == SlideKind::Part && !context.slide.terminals.is_empty() {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    "part slide declares an embedded terminal",
                )
                .with_help("move the terminal to a standard slide"),
            );
        }
    }
}

/// A slide has terminals but no working directory resolves for the quake overlay.
pub(crate) struct UnresolvedCwd;

impl Rule for UnresolvedCwd {
    fn id(&self) -> RuleId {
        super::ids::TERM_UNRESOLVED_CWD
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        if context.slide.terminals.is_empty() {
            return;
        }
        if context.slide.resolved_quake_cwd(context.talk).is_none() {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    "terminal slide has no resolved working directory",
                )
                .with_help(
                    "set the slide's quake terminal cwd or the talk default_terminal_cwd; \
                     otherwise the server's own cwd is used",
                ),
            );
        }
    }
}

/// Multiple terminals on one slide share an identical working directory.
pub(crate) struct DuplicateCwd;

impl Rule for DuplicateCwd {
    fn id(&self) -> RuleId {
        super::ids::TERM_DUPLICATE_CWD
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let mut seen = HashSet::new();
        let has_duplicate = context
            .slide
            .terminals
            .iter()
            .any(|terminal| !seen.insert(terminal.cwd.clone()));
        if has_duplicate {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    "multiple terminals share the same working directory",
                )
                .with_help("give each terminal a distinct cwd, or use a single terminal"),
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_core::{Slide, TerminalConfig};

    use super::*;
    use crate::diagnostic::SlideRef;
    use crate::rule::{LintConfig, RuleContext};
    use crate::rules::test_support::{fires, slide_diagnostics};

    /// Runs `rule` over `slide` as part of `talk`, which these rules consult
    /// for the deck-level terminal default.
    fn diagnostics_in(rule: &dyn Rule, talk: &toboggan_core::Talk) -> Vec<LintDiagnostic> {
        let slide = talk.slides.first().expect("a slide");
        let slide_ref = SlideRef::new(0, slide);
        let config = LintConfig::default();
        let context = RuleContext::new(talk, slide, &slide_ref, &config);
        let mut out = Vec::new();
        rule.check_slide(&context, &mut out);
        out
    }

    fn talk_with(slide: Slide) -> toboggan_core::Talk {
        let mut talk = toboggan_core::Talk::new("Test");
        talk.slides = vec![slide];
        talk
    }

    #[test]
    fn a_part_with_a_terminal_is_reported() {
        let part = Slide::part("Section").with_terminal(TerminalConfig::new("/tmp"));
        assert!(fires(&TerminalInPart, &part));
    }

    /// A terminal on a content slide is the normal case and must stay quiet;
    /// so must a part slide that simply has none.
    #[test]
    fn terminals_elsewhere_are_fine() {
        let standard = Slide::new("T").with_terminal(TerminalConfig::new("/tmp"));
        assert!(!fires(&TerminalInPart, &standard));
        assert!(!fires(&TerminalInPart, &Slide::part("Section")));
    }

    /// Without a cwd anywhere, the server silently falls back to its own
    /// working directory — which is wherever the presenter happened to launch
    /// it, and never what the slide meant.
    #[test]
    fn a_terminal_with_no_cwd_anywhere_is_reported() {
        let slide = Slide::new("T").with_terminal(TerminalConfig::new("/tmp"));
        assert_eq!(diagnostics_in(&UnresolvedCwd, &talk_with(slide)).len(), 1);
    }

    #[test]
    fn a_slide_cwd_resolves() {
        let slide = Slide::new("T")
            .with_terminal(TerminalConfig::new("/tmp"))
            .with_quake_terminal_cwd("/tmp/demo");
        assert!(diagnostics_in(&UnresolvedCwd, &talk_with(slide)).is_empty());
    }

    /// The deck-level default is the whole reason this rule needs the talk and
    /// not just the slide.
    #[test]
    fn the_talk_default_resolves_too() {
        let slide = Slide::new("T").with_terminal(TerminalConfig::new("/tmp"));
        let mut talk = talk_with(slide);
        talk.default_terminal_cwd = Some("/tmp/demo".to_owned());
        assert!(diagnostics_in(&UnresolvedCwd, &talk).is_empty());
    }

    /// A slide with no terminals has nothing to resolve, so the rule must not
    /// complain about the missing cwd it does not need.
    #[test]
    fn a_slide_without_terminals_is_not_asked_for_a_cwd() {
        assert!(diagnostics_in(&UnresolvedCwd, &talk_with(Slide::new("T"))).is_empty());
    }

    /// Two terminals on one cwd render two panes onto the same directory,
    /// which is a copy-paste artifact rather than a layout anyone wants.
    #[test]
    fn two_terminals_on_one_directory_are_reported() {
        let slide = Slide::new("T")
            .with_terminal(TerminalConfig::new("/tmp/demo"))
            .with_terminal(TerminalConfig::new("/tmp/demo"));
        assert_eq!(slide_diagnostics(&DuplicateCwd, &slide).len(), 1);
    }

    #[test]
    fn distinct_directories_are_fine() {
        let slide = Slide::new("T")
            .with_terminal(TerminalConfig::new("/tmp/one"))
            .with_terminal(TerminalConfig::new("/tmp/two"));
        assert!(!fires(&DuplicateCwd, &slide));
    }
}
