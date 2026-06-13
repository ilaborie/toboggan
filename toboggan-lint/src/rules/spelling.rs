use std::process::Command;

use toboggan_core::Talk;

use crate::diagnostic::{LintDiagnostic, RuleId, Severity};

const RULE: RuleId = RuleId("spelling/typo");

/// Runs the `typos` CLI over the talk's source directory and maps each hit to an
/// info-level diagnostic. Honors the repo's `typos.toml` automatically.
///
/// Degrades gracefully: if the talk has no source directory, or the `typos`
/// binary is not installed, a single explanatory info diagnostic is returned
/// instead of failing.
#[must_use]
pub fn spell_check(talk: &Talk) -> Vec<LintDiagnostic> {
    let Some(dir) = talk.source_dir.as_deref() else {
        return vec![LintDiagnostic::talk(
            RULE,
            Severity::Info,
            "spell check skipped: talk has no source directory",
        )];
    };

    let output = Command::new("typos")
        .arg("--format")
        .arg("brief")
        .arg(dir)
        .output();

    match output {
        Err(_) => vec![LintDiagnostic::talk(
            RULE,
            Severity::Info,
            "spell check skipped: `typos` CLI not found on PATH",
        )],
        Ok(result) => String::from_utf8_lossy(&result.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| {
                LintDiagnostic::talk(RULE, Severity::Info, line.to_owned())
                    .with_help("fix the typo or add it to typos.toml")
            })
            .collect(),
    }
}
