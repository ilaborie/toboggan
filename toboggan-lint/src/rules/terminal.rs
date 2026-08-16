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
