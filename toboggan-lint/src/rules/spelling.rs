use std::process::Command;

use toboggan_core::Talk;

use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{LintConfig, Rule};

/// Spell-checks the talk's source with the `typos` CLI (honors the repo's
/// `typos.toml`), mapping each hit to an info-level diagnostic.
///
/// Runs as part of the default suite when the `spell` feature is enabled — for
/// the library and `toboggan lint`; the MCP `lint` tool disables it unless its
/// `spell` parameter is set.
///
/// Degrades to an **info diagnostic**, never an error: a missing `typos` binary
/// or a failing run must not turn an otherwise-clean deck into a failing one.
/// But it does not degrade to *silence* — reporting "no typos" when the check
/// never ran is indistinguishable from a clean deck, which is exactly the
/// mistake this rule exists to prevent.
pub(crate) struct SpellCheck;

impl Rule for SpellCheck {
    fn id(&self) -> RuleId {
        super::ids::SPELLING_TYPO
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_talk(&self, talk: &Talk, _config: &LintConfig, out: &mut Vec<LintDiagnostic>) {
        let Some(dir) = talk.source_dir.as_deref() else {
            return;
        };
        let result = match Command::new("typos")
            .arg("--format")
            .arg("brief")
            .arg(dir)
            .output()
        {
            Ok(result) => result,
            Err(err) => {
                out.push(
                    LintDiagnostic::talk(
                        self.id(),
                        Severity::Info,
                        format!("spell check skipped: could not run `typos` ({err})"),
                    )
                    .with_help("install https://github.com/crate-ci/typos, or pass --no-spell"),
                );
                return;
            }
        };
        // `typos` exits non-zero when it finds typos, so a non-zero status is only
        // a failure when it also wrote to stderr (bad config, unreadable dir).
        // Without this the deck came back clean from a run that never checked it.
        if !result.status.success() && !result.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            out.push(
                LintDiagnostic::talk(
                    self.id(),
                    Severity::Info,
                    format!("spell check failed ({}): {}", result.status, stderr.trim()),
                )
                .with_help("check typos.toml, or pass --no-spell"),
            );
            return;
        }
        for line in String::from_utf8_lossy(&result.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            out.push(
                LintDiagnostic::talk(self.id(), self.default_severity(), line.to_owned())
                    .with_help("fix the typo or add it to typos.toml"),
            );
        }
    }
}
