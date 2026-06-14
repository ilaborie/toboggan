use std::process::Command;

use toboggan_core::Talk;

use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{LintConfig, Rule};

/// Spell-checks the talk's source with the `typos` CLI (honors the repo's
/// `typos.toml`), mapping each hit to an info-level diagnostic.
///
/// Runs as part of the default suite when the `spell` feature is enabled.
/// Degrades **silently**: if the talk has no source directory or the `typos`
/// binary is not installed, it emits nothing — best-effort spell checking must
/// never turn an otherwise-clean deck into a failing one.
pub(crate) struct SpellCheck;

impl Rule for SpellCheck {
    fn id(&self) -> RuleId {
        RuleId("spelling/typo")
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_talk(&self, talk: &Talk, _config: &LintConfig, out: &mut Vec<LintDiagnostic>) {
        let Some(dir) = talk.source_dir.as_deref() else {
            return;
        };
        let Ok(result) = Command::new("typos")
            .arg("--format")
            .arg("brief")
            .arg(dir)
            .output()
        else {
            return;
        };
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
