use std::io::Write as _;

use owo_colors::{OwoColorize, Stream};
use toboggan_lint::{LintDiagnostic, LintReport, Severity};

use crate::cli::{DenyLevel, LintFormat, ResolvedLint};

/// Lints a presentation folder, prints the report, and fails if any diagnostic
/// meets the `--deny` threshold.
///
/// # Errors
/// Returns an error if the folder cannot be parsed, or if diagnostics reach the
/// deny threshold.
#[allow(clippy::print_stdout)]
pub(crate) fn run_lint(resolved: ResolvedLint) -> anyhow::Result<()> {
    let ResolvedLint {
        input,
        mut settings,
        deny,
        format,
        lint: config,
    } = resolved;

    let slides = super::deck::resolve_deck(&input).slides;
    settings.input = Some(slides.clone());
    let talk = super::deck::build_talk(&slides, &settings)?;

    let report = toboggan_lint::lint(&talk, &config);

    match format {
        LintFormat::Human => print_report(&report),
        LintFormat::Json => print_json(&report)?,
        LintFormat::Github => print_github(&report)?,
        LintFormat::Sarif => print_sarif(&report)?,
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
    println!(
        "{label} [{rule}] {location}: {message}",
        rule = diagnostic.rule.as_str(),
        location = location_of(diagnostic),
        message = diagnostic.message,
    );
    if let Some(help) = &diagnostic.help {
        println!("      help: {help}");
    }
}

/// Names where a diagnostic came from.
///
/// Prefers the source file, because that is what the reader has to open — and
/// what an editor or terminal will turn into a clickable link. The slide number
/// stays as a suffix so a deck's ordering is still visible, and is all there is
/// for a slide with no file of its own.
fn location_of(diagnostic: &LintDiagnostic) -> String {
    let slide = diagnostic
        .slide
        .as_ref()
        .map(|slide| format!("slide {} \"{}\"", slide.display_number(), slide.title));
    match (&diagnostic.source_path, slide) {
        (Some(path), Some(slide)) => format!("{} ({slide})", path.display()),
        (Some(path), None) => path.display().to_string(),
        (None, Some(slide)) => slide,
        (None, None) => "talk".to_owned(),
    }
}

fn print_json(report: &LintReport) -> anyhow::Result<()> {
    let output = serde_json::to_string_pretty(report)
        .map_err(|err| anyhow::anyhow!("serializing report: {err}"))?;
    write_line(&output)
}

/// Writes one line to stdout, turning a closed pipe into a clean exit.
///
/// `println!` panics if stdout is gone, so `toboggan lint --format json | head`
/// ended in a panic message rather than the output the user asked for. The
/// machine-readable formats are the ones people pipe, which is why they are the
/// ones that hit it.
fn write_line(line: &str) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout().lock();
    match writeln!(stdout, "{line}") {
        Ok(()) => Ok(()),
        // The reader went away — `head`, `grep -q`, a closed pager. That is the
        // pipeline working, not a failure to report.
        Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(err) => Err(anyhow::anyhow!("writing to stdout: {err}")),
    }
}

/// Prints GitHub Actions workflow commands, which the runner turns into inline
/// annotations on a pull request.
///
/// Only useful because diagnostics now carry a file. Without `line`, GitHub
/// pins the annotation to the top of the file, which is right: a diagnostic
/// covers a whole slide, and slides are one file each.
fn print_github(report: &LintReport) -> anyhow::Result<()> {
    for line in github_lines(report) {
        write_line(&line)?;
    }
    Ok(())
}

/// Builds the workflow command for each diagnostic.
fn github_lines(report: &LintReport) -> Vec<String> {
    report
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let level = match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                // GitHub's third level is "notice", not "info".
                Severity::Info => "notice",
            };
            let file = diagnostic
                .source_path
                .iter()
                .map(|path| format!("file={}", escape_property(&path.display().to_string())));
            let properties = file
                .chain(std::iter::once(format!(
                    "title={}",
                    escape_property(diagnostic.rule.as_str())
                )))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "::{level} {properties}::{}",
                escape_data(&message_with_help(diagnostic))
            )
        })
        .collect()
}

/// Prints a SARIF 2.1.0 log, which GitHub code scanning and other analysis
/// tools ingest directly.
fn print_sarif(report: &LintReport) -> anyhow::Result<()> {
    let output = serde_json::to_string_pretty(&sarif_log(report))
        .map_err(|err| anyhow::anyhow!("serializing SARIF: {err}"))?;
    write_line(&output)
}

/// Builds the SARIF log for `report`.
fn sarif_log(report: &LintReport) -> serde_json::Value {
    let results = report
        .diagnostics
        .iter()
        .map(|diagnostic| {
            // An empty array rather than a missing key: SARIF spells "this
            // result is not about a place in a file" as no locations, which is
            // exactly a talk-level diagnostic.
            let locations = diagnostic
                .source_path
                .iter()
                .map(|path| {
                    serde_json::json!({
                        "physicalLocation": {
                            "artifactLocation": { "uri": path.display().to_string() },
                        },
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "ruleId": diagnostic.rule.as_str(),
                "level": match diagnostic.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    // SARIF's third level is "note".
                    Severity::Info => "note",
                },
                "message": { "text": message_with_help(diagnostic) },
                "locations": locations,
            })
        })
        .collect::<Vec<_>>();

    // Every rule this build knows, not just the ones that fired: a consumer
    // reads the catalog to render a rule's name even for a result it filters.
    let rules = toboggan_lint::all_rule_ids()
        .into_iter()
        .map(|id| serde_json::json!({ "id": id.as_str(), "name": id.as_str() }))
        .collect::<Vec<_>>();

    serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": {
                "name": "toboggan-lint",
                "informationUri": "https://github.com/ilaborie/toboggan",
                "version": env!("CARGO_PKG_VERSION"),
                "rules": rules,
            }},
            "results": results,
        }],
    })
}

/// The message with its help appended, for formats with no separate help field.
fn message_with_help(diagnostic: &LintDiagnostic) -> String {
    match &diagnostic.help {
        Some(help) => format!("{} — {help}", diagnostic.message),
        None => diagnostic.message.clone(),
    }
}

/// Escapes a workflow command's message.
///
/// A raw newline would end the command and print the rest as plain log output,
/// losing the annotation; a `%` would be read as the start of an escape.
fn escape_data(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escapes a workflow command's property value, which additionally cannot
/// contain the `,` and `:` that separate properties from the message.
fn escape_property(value: &str) -> String {
    escape_data(value).replace(',', "%2C").replace(':', "%3A")
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

    /// A raw newline ends a workflow command, so an unescaped one turns the
    /// annotation into ordinary log output and the finding vanishes from the
    /// pull request. `%` has to go first or it would double-escape the others.
    #[test]
    fn workflow_command_data_is_escaped() {
        assert_eq!(escape_data("100% done\nnext"), "100%25 done%0Anext");
        assert_eq!(escape_data("a\r\nb"), "a%0D%0Ab");
    }

    /// A property value additionally cannot carry the separators, or the runner
    /// reads the rest of a path as another property.
    #[test]
    fn workflow_command_properties_escape_the_separators() {
        assert_eq!(
            escape_property("C:/decks/a,b/slide.md"),
            "C%3A/decks/a%2Cb/slide.md"
        );
    }

    /// GitHub's levels are error/warning/notice — not the linter's own names,
    /// and an unrecognized one is silently dropped by the runner.
    #[test]
    fn github_levels_map_to_the_ones_github_knows() {
        let diagnostic = |severity| {
            LintDiagnostic::slide(
                toboggan_lint::ids::PAUSE_IN_PART,
                severity,
                &toboggan_lint::SlideRef {
                    index: 0,
                    title: "T".to_owned(),
                },
                "boom",
            )
            .with_source_path("slides/a.md")
        };
        let report = LintReport::new(vec![
            diagnostic(Severity::Error),
            diagnostic(Severity::Warning),
            diagnostic(Severity::Info),
        ]);
        let lines = github_lines(&report);
        assert_eq!(
            lines,
            vec![
                "::error file=slides/a.md,title=pause/in-part::boom",
                "::warning file=slides/a.md,title=pause/in-part::boom",
                "::notice file=slides/a.md,title=pause/in-part::boom",
            ]
        );
    }

    /// A talk-level diagnostic has no file; the command still has to be
    /// well-formed, or the runner drops it.
    #[test]
    fn a_diagnostic_without_a_file_still_annotates() {
        let report = report_with(&[Severity::Warning]);
        assert_eq!(
            github_lines(&report),
            vec!["::warning title=pause/in-part::x"]
        );
    }

    #[test]
    fn sarif_carries_the_location_and_the_rule_catalog() {
        let report = LintReport::new(vec![
            LintDiagnostic::talk(toboggan_lint::ids::PAUSE_IN_PART, Severity::Info, "x")
                .with_source_path("slides/a.md"),
        ]);
        let log = sarif_log(&report);
        let run = log
            .pointer("/runs/0")
            .expect("a SARIF log has exactly one run");
        assert_eq!(
            run.pointer("/results/0/level").and_then(|it| it.as_str()),
            Some("note"),
            "SARIF's third level is `note`"
        );
        assert_eq!(
            run.pointer("/results/0/locations/0/physicalLocation/artifactLocation/uri")
                .and_then(|it| it.as_str()),
            Some("slides/a.md")
        );
        let rules = run
            .pointer("/tool/driver/rules")
            .and_then(|it| it.as_array())
            .expect("a rule catalog");
        assert_eq!(
            rules.len(),
            toboggan_lint::all_rule_ids().len(),
            "the catalog lists every rule, not only the ones that fired"
        );
    }
}
