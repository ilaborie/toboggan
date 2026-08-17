use crate::diagnostic::{LintDiagnostic, RuleId, Severity};
use crate::rule::{Rule, RuleContext};

/// A code block with more lines than fit on a projected slide.
pub(crate) struct CodeTooLong;

impl Rule for CodeTooLong {
    fn id(&self) -> RuleId {
        super::ids::CODE_TOO_LONG
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let max = context.config.max_code_lines;
        for block in context.body_doc().code_blocks() {
            if block.lines > max {
                out.push(
                    LintDiagnostic::slide(
                        self.id(),
                        self.default_severity(),
                        context.slide_ref,
                        format!(
                            "code block is {} lines (suggested limit {max})",
                            block.lines
                        ),
                    )
                    .with_help(
                        "show the part that matters — the rest is unreadable from the back row",
                    ),
                );
            }
        }
    }
}

/// A fenced code block with no language, so it is neither highlighted nor
/// exported as code.
pub(crate) struct CodeNoLanguage;

impl Rule for CodeNoLanguage {
    fn id(&self) -> RuleId {
        super::ids::CODE_NO_LANGUAGE
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check_slide(&self, context: &RuleContext<'_>, out: &mut Vec<LintDiagnostic>) {
        let count = context
            .body_doc()
            .code_blocks()
            .iter()
            .filter(|block| block.language.is_none())
            .count();
        if count > 0 {
            let plural = if count == 1 { "block" } else { "blocks" };
            out.push(
                LintDiagnostic::slide(
                    self.id(),
                    self.default_severity(),
                    context.slide_ref,
                    format!("{count} code {plural} with no language"),
                )
                .with_help("tag the fence, e.g. ```rust — an untagged block is not highlighted"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use toboggan_core::{Content, Slide};

    use super::*;
    use crate::rule::LintConfig;
    use crate::rules::test_support::{fires, only, slide_diagnostics, slide_diagnostics_with};

    fn slide_with(body: &str) -> Slide {
        Slide::new("T").with_body(Content::html(body))
    }

    fn code_block(lines: usize) -> String {
        let source = "x\n".repeat(lines);
        format!(r#"<pre><code class="language-rust">{source}</code></pre>"#)
    }

    /// Highlighted output is `<pre><code class="language-rust">`; an untagged
    /// fence is `<pre><code>`. The rule reads the class, not the markdown.
    #[test]
    fn an_untagged_fence_is_reported() {
        assert!(fires(
            &CodeNoLanguage,
            &slide_with("<pre><code>plain text</code></pre>")
        ));
    }

    #[test]
    fn a_tagged_fence_is_not() {
        let slide = slide_with(r#"<pre><code class="language-rust">fn main() {}</code></pre>"#);
        assert!(!fires(&CodeNoLanguage, &slide));
    }

    /// Inline `<code>` is not a code block and must not be reported — a slide
    /// mentioning `cargo build` in prose would otherwise trip the rule.
    #[test]
    fn inline_code_is_not_a_block() {
        let slide = slide_with("<p>run <code>cargo build</code> first</p>");
        assert!(!fires(&CodeNoLanguage, &slide));
    }

    #[test]
    fn one_diagnostic_counts_every_untagged_block() {
        let slide = slide_with(
            r#"<pre><code>one</code></pre>
               <pre><code class="language-rust">two</code></pre>
               <pre><code>three</code></pre>"#,
        );
        let diagnostics = slide_diagnostics(&CodeNoLanguage, &slide);
        let message = &only(&diagnostics).message;
        assert!(message.starts_with("2 code blocks"), "{message}");
    }

    /// The threshold is `>`, so a block sitting exactly on the limit stays
    /// quiet. A `>` → `>=` slip is invisible without both halves.
    #[test]
    fn too_long_fires_only_above_the_limit() {
        let config = LintConfig {
            max_code_lines: 3,
            ..LintConfig::default()
        };
        let at_limit = slide_with(&code_block(3));
        assert!(slide_diagnostics_with(&CodeTooLong, &at_limit, &config).is_empty());

        let over = slide_with(&code_block(4));
        assert_eq!(
            slide_diagnostics_with(&CodeTooLong, &over, &config).len(),
            1
        );
    }

    /// Each over-long block is its own finding: a slide with two of them has
    /// two problems, and reporting one hides the other.
    #[test]
    fn every_over_long_block_is_reported() {
        let config = LintConfig {
            max_code_lines: 2,
            ..LintConfig::default()
        };
        let slide = slide_with(&format!("{}{}", code_block(5), code_block(6)));
        assert_eq!(
            slide_diagnostics_with(&CodeTooLong, &slide, &config).len(),
            2
        );
    }
}
