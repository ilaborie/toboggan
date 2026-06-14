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

    let settings = build.into_cli_settings(input.clone(), true);
    let parse_result = toboggan_cli::parse_presentation(&input, &settings)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let talk = parse_result.to_talk();

    // Spell checking (`spelling/typo`) runs by default; `--no-spell` opts out.
    let mut config = LintConfig::default();
    if no_spell {
        config.disabled.insert("spelling/typo".to_owned());
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
            report.errors,
            report.warnings,
            report.infos
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
        report.errors, report.warnings, report.infos
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
        DenyLevel::Error => report.errors > 0,
        DenyLevel::Warning => report.errors > 0 || report.warnings > 0,
        DenyLevel::Info => !report.is_clean(),
    }
}
