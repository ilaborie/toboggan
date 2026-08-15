use owo_colors::{OwoColorize, Stream};
use toboggan_lint::{LintConfig, LintDiagnostic, LintReport, Severity};

use crate::cli::{DenyLevel, LintArgs};

/// Lints a presentation folder, prints the report, and fails if any diagnostic
/// meets the `--deny` threshold.
///
/// # Errors
/// Returns an error if the folder cannot be parsed, or if diagnostics reach the
/// deny threshold.
#[allow(clippy::print_stdout)]
pub(crate) fn run_lint(args: LintArgs) -> anyhow::Result<()> {
    let LintArgs {
        input,
        deny,
        json,
        no_spell,
        build,
    } = args;

    let slides = super::deck::resolve_deck(&input).slides;
    let settings = build.into_cli_settings(slides.clone(), true);
    let talk = super::deck::build_talk(&slides, &settings)?;

    // Spell checking (`spelling/typo`) runs by default; `--no-spell` opts out.
    let mut config = LintConfig::default();
    if no_spell {
        config.disable(toboggan_lint::ids::SPELLING_TYPO);
    }
    let report = toboggan_lint::lint(&talk, &config);

    if json {
        let output = serde_json::to_string_pretty(&report)
            .map_err(|err| anyhow::anyhow!("serializing report: {err}"))?;
        println!("{output}");
    } else {
        print_report(&report);
    }

    if reaches_threshold(&report, deny) {
        anyhow::bail!(
            "lint failed: {} error(s), {} warning(s), {} info(s)",
            report.errors(),
            report.warnings(),
            report.infos()
        );
    }
    Ok(())
}

#[allow(clippy::print_stdout)]
fn print_report(report: &LintReport) {
    if report.is_clean() {
        println!(
            "{} no lint issues found",
            "✓".if_supports_color(Stream::Stdout, |text| text.green())
        );
        return;
    }

    for diagnostic in &report.diagnostics {
        print_diagnostic(diagnostic);
    }

    println!(
        "\n{} error(s), {} warning(s), {} info(s)",
        report.errors(),
        report.warnings(),
        report.infos()
    );
}

#[allow(clippy::print_stdout)]
fn print_diagnostic(diagnostic: &LintDiagnostic) {
    let label = severity_label(diagnostic.severity);
    let location = match &diagnostic.slide {
        Some(slide) => format!("slide {} \"{}\"", slide.display_number, slide.title),
        None => "talk".to_owned(),
    };
    println!(
        "{label} [{rule}] {location}: {message}",
        rule = diagnostic.rule.as_str(),
        message = diagnostic.message,
    );
    if let Some(help) = &diagnostic.help {
        println!("      help: {help}");
    }
}

fn severity_label(severity: Severity) -> String {
    match severity {
        Severity::Error => format!(
            "{}",
            "error".if_supports_color(Stream::Stdout, OwoColorize::red)
        ),
        Severity::Warning => format!(
            "{}",
            "warn ".if_supports_color(Stream::Stdout, OwoColorize::yellow)
        ),
        Severity::Info => format!(
            "{}",
            "info ".if_supports_color(Stream::Stdout, OwoColorize::blue)
        ),
    }
}

fn reaches_threshold(report: &LintReport, deny: DenyLevel) -> bool {
    match deny {
        DenyLevel::Error => report.errors() > 0,
        DenyLevel::Warning => report.errors() > 0 || report.warnings() > 0,
        DenyLevel::Info => !report.is_clean(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_lint::{LintDiagnostic, LintReport, Severity};

    use super::*;

    fn report_with(severities: &[Severity]) -> LintReport {
        let diagnostics = severities
            .iter()
            .map(|severity| {
                LintDiagnostic::talk(toboggan_lint::ids::PAUSE_IN_PART, *severity, "x".to_owned())
            })
            .collect();
        LintReport::new(diagnostics)
    }

    /// This function decides `toboggan lint`'s exit code, which is what CI gates
    /// on — so every arm is pinned. Swapping `errors` for `warnings` in the
    /// `Error` arm would make every CI run green on a deck with real errors.
    #[test]
    fn deny_error_fires_only_on_errors() {
        let deny = DenyLevel::Error;
        assert!(!reaches_threshold(&report_with(&[]), deny));
        assert!(!reaches_threshold(&report_with(&[Severity::Info]), deny));
        assert!(!reaches_threshold(&report_with(&[Severity::Warning]), deny));
        assert!(reaches_threshold(&report_with(&[Severity::Error]), deny));
    }

    #[test]
    fn deny_warning_fires_on_warnings_and_errors() {
        let deny = DenyLevel::Warning;
        assert!(!reaches_threshold(&report_with(&[]), deny));
        assert!(!reaches_threshold(&report_with(&[Severity::Info]), deny));
        assert!(reaches_threshold(&report_with(&[Severity::Warning]), deny));
        assert!(reaches_threshold(&report_with(&[Severity::Error]), deny));
    }

    #[test]
    fn deny_info_fires_on_any_diagnostic() {
        let deny = DenyLevel::Info;
        assert!(!reaches_threshold(&report_with(&[]), deny));
        assert!(reaches_threshold(&report_with(&[Severity::Info]), deny));
        assert!(reaches_threshold(&report_with(&[Severity::Error]), deny));
    }
}
