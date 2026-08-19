use std::collections::HashSet;
use std::path::Path;

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

/// A terminal's working directory cannot be resolved to a real place.
///
/// This used to guard on `slide.terminals` and then check
/// [`Slide::resolved_quake_cwd`], which are two unrelated features: the quake
/// overlay's cwd is never read from the embedded terminals, and
/// `TerminalConfig::cwd` is not optional — every embedded terminal has one. So
/// the rule reported "no resolved working directory" for slides whose terminals
/// all declared one, and fired on three of this project's own examples.
///
/// What is genuinely unresolvable is a *relative* cwd on a talk with no
/// `source_dir` to join it against: the terminal then opens wherever the server
/// happens to have been started, which is not a place the author chose.
pub(crate) struct UnresolvedCwd;

impl Rule for UnresolvedCwd {
    fn id(&self) -> RuleId {
        super::ids::TERM_UNRESOLVED_CWD
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        // An absolute cwd needs nothing; a relative one needs a deck root.
        if context.talk.source_dir.is_some() {
            return;
        }
        let unresolvable = context
            .slide
            .terminals
            .iter()
            .any(|terminal| terminal.cwd.is_relative())
            || context
                .slide
                .quake_terminal_cwd
                .as_deref()
                .or(context.talk.default_terminal_cwd.as_deref())
                .is_some_and(|cwd| Path::new(cwd).is_relative());

        if unresolvable {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    "terminal working directory is relative and the deck has no root to resolve it against",
                )
                .with_help(
                    "use an absolute cwd, or build the deck from its folder so the \
                     relative path resolves; otherwise the server's own cwd is used",
                ),
            );
        }
    }
}

/// Multiple terminals on one slide would run the identical thing.
pub(crate) struct DuplicateCwd;

impl Rule for DuplicateCwd {
    fn id(&self) -> RuleId {
        super::ids::TERM_DUPLICATE_CWD
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        // Keyed on the directory *and* the command: two terminals side by side
        // in one repo, one running the tests and one an editor, is the feature
        // `06-terminal-multi.md` demonstrates — not a mistake to report.
        let mut seen = HashSet::new();
        let has_duplicate = context
            .slide
            .terminals
            .iter()
            .any(|terminal| !seen.insert((&terminal.cwd, &terminal.cmd)));
        if has_duplicate {
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    "two terminals on this slide run the same command in the same directory",
                )
                .with_help("give each terminal its own command or cwd, or use a single terminal"),
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
    use crate::rules::test_support::{fires, only, slide_diagnostics, talk_of};

    /// Runs `rule` over the talk's first slide. These rules consult the talk
    /// for its deck root and terminal default, which `slide_diagnostics` builds
    /// fresh — so they need the talk the caller set up.
    fn diagnostics_in(rule: &dyn Rule, talk: &toboggan_core::Talk) -> Vec<LintDiagnostic> {
        let slide = talk.slides.first().expect("a slide");
        let slide_ref = SlideRef::new(0, slide);
        let config = LintConfig::default();
        let context = RuleContext::new(talk, slide, &slide_ref, &config);
        let mut out = Vec::new();
        rule.check_slide(&context, &mut out);
        out
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

    /// A relative cwd with no deck root behind it opens the terminal wherever
    /// the server was launched, which is never what the slide meant.
    #[test]
    fn a_relative_cwd_with_no_deck_root_is_reported() {
        let slide = Slide::new("T").with_terminal(TerminalConfig::new("."));
        assert_eq!(diagnostics_in(&UnresolvedCwd, &talk_of(&[slide])).len(), 1);
    }

    /// The everyday case, and the one this rule used to report: a deck parsed
    /// from its folder has a root, so `.` resolves against it. Three of this
    /// project's own examples were flagged for exactly this.
    #[test]
    fn a_relative_cwd_resolves_against_the_deck_root() {
        let slide = Slide::new("T").with_terminal(TerminalConfig::new("."));
        let mut talk = talk_of(&[slide]);
        talk.source_dir = Some("/talks/demo".to_owned());
        assert!(diagnostics_in(&UnresolvedCwd, &talk).is_empty());
    }

    /// An absolute cwd needs no deck root at all.
    #[test]
    fn an_absolute_cwd_needs_nothing() {
        let slide = Slide::new("T").with_terminal(TerminalConfig::new("/tmp/demo"));
        assert!(diagnostics_in(&UnresolvedCwd, &talk_of(&[slide])).is_empty());
    }

    /// The quake overlay's cwd is resolved the same way, and it is the reason
    /// this rule needs the talk rather than just the slide.
    #[test]
    fn a_relative_quake_cwd_is_reported_too() {
        let slide = Slide::new("T").with_quake_terminal_cwd("examples/api");
        assert_eq!(diagnostics_in(&UnresolvedCwd, &talk_of(&[slide])).len(), 1);

        let mut talk = talk_of(&[Slide::new("T")]);
        talk.default_terminal_cwd = Some("examples/api".to_owned());
        assert_eq!(diagnostics_in(&UnresolvedCwd, &talk).len(), 1);
    }

    /// A slide with no terminal and no overlay cwd has nothing to resolve.
    #[test]
    fn a_slide_without_terminals_is_not_asked_for_a_cwd() {
        assert!(diagnostics_in(&UnresolvedCwd, &talk_of(&[Slide::new("T")])).is_empty());
    }

    /// Two panes running the same command in the same directory is a
    /// copy-paste artifact rather than a layout anyone wants.
    #[test]
    fn two_identical_terminals_are_reported() {
        let slide = Slide::new("T")
            .with_terminal(TerminalConfig::new("/tmp/demo"))
            .with_terminal(TerminalConfig::new("/tmp/demo"));
        let diagnostics = slide_diagnostics(&DuplicateCwd, &slide);
        assert!(
            only(&diagnostics).message.contains("same command"),
            "{diagnostics:#?}"
        );
    }

    /// The side-by-side feature: one repo, the tests in one pane and an editor
    /// in the other. This used to be reported, on the example that demonstrates
    /// it — the rule keyed on the directory alone.
    #[test]
    fn two_commands_in_one_directory_are_fine() {
        let slide = Slide::new("T")
            .with_terminal(TerminalConfig::new(".").with_cmd("bacon test"))
            .with_terminal(TerminalConfig::new(".").with_cmd("hx src/lib.rs"));
        assert!(!fires(&DuplicateCwd, &slide));
    }

    #[test]
    fn distinct_directories_are_fine() {
        let slide = Slide::new("T")
            .with_terminal(TerminalConfig::new("/tmp/one"))
            .with_terminal(TerminalConfig::new("/tmp/two"));
        assert!(!fires(&DuplicateCwd, &slide));
    }
}
