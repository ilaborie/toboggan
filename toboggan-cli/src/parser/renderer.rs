use std::fmt::Write;

use comrak::nodes::{AstNode, NodeValue};
use comrak::options::Plugins;
use comrak::{Arena, Options, format_html_with_plugins, parse_document};
use math_core::{LatexToMathML, MathCoreConfig, MathDisplay};
use toboggan_core::{Content, Style};

use super::CssClasses;
use crate::error::{Result, TobogganCliError};

pub(super) trait ContentRenderer {
    fn render_steps(&self, before_steps: &str, steps: &[(String, CssClasses)]) -> Result<Content>;
}

pub(super) struct HtmlRenderer<'a> {
    options: &'a Options<'a>,
    plugins: &'a Plugins<'a>,
    style: Style,
    /// Slide file the markdown came from, used to name the file in a math error.
    source_name: &'a str,
}

impl<'a> HtmlRenderer<'a> {
    #[must_use]
    pub(super) fn new(
        options: &'a Options<'_>,
        plugins: &'a Plugins<'_>,
        style: Style,
        source_name: &'a str,
    ) -> Self {
        Self {
            options,
            plugins,
            style,
            source_name,
        }
    }

    /// Renders one markdown block, converting `$…$` / `$$…$$` to `MathML` first.
    #[allow(clippy::expect_used)]
    fn render_markdown(&self, markdown: &str) -> Result<String> {
        let arena = Arena::new();
        let root = parse_document(&arena, markdown, self.options);
        self.inline_math_as_mathml(root)?;

        let mut html = String::new();
        format_html_with_plugins(root, self.options, &mut html, self.plugins)
            .expect("Writing to a String should never fail");
        Ok(html)
    }

    /// Replaces every math node with the `MathML` it denotes.
    ///
    /// comrak's own renderer emits `<span data-math-style="…">` holding the raw
    /// LaTeX, which is only useful with a client-side renderer such as `KaTeX`.
    /// Converting here instead keeps the output self-contained: no script, no
    /// CDN and no web font, so an exported deck renders its math offline, in a
    /// print preview, and in the embedded web client alike — all of which read
    /// the same HTML.
    fn inline_math_as_mathml<'arena>(&self, node: &'arena AstNode<'arena>) -> Result<()> {
        // `math_dollars` is the only math extension enabled, so every math node
        // here came from `$`-delimited source.
        let converted = {
            let mut data = node.data.borrow_mut();
            match &data.value {
                NodeValue::Math(math) => {
                    let display = if math.display_math {
                        MathDisplay::Block
                    } else {
                        MathDisplay::Inline
                    };
                    Some(NodeValue::HtmlInline(
                        self.latex_to_mathml(&math.literal, display)?,
                    ))
                }
                _ => None,
            }
            .map(|value| data.value = value)
        };
        // Math nodes hold their contents in `literal` rather than as children,
        // so a converted node has nothing left to walk into.
        if converted.is_none() {
            for child in node.children() {
                self.inline_math_as_mathml(child)?;
            }
        }
        Ok(())
    }

    fn latex_to_mathml(&self, latex: &str, display: MathDisplay) -> Result<String> {
        // Cheap to build (the default config defines no macros, which is the
        // only thing `new` can reject), and math is rare enough that hoisting
        // the converter into `HtmlRenderer` would buy nothing.
        let converter =
            LatexToMathML::new(MathCoreConfig::default()).map_err(|(err, index, definition)| {
                self.math_error(latex, &format!("macro {index} (`{definition}`): {err}"))
            })?;
        converter
            .convert_with_local_state(latex, display)
            .map(|result| result.mathml)
            .map_err(|err| self.math_error(latex, &err.to_string()))
    }

    fn math_error(&self, latex: &str, reason: &str) -> TobogganCliError {
        TobogganCliError::InvalidMath {
            file: self.source_name.to_owned(),
            latex: latex.to_owned(),
            message: format!("`{latex}` — {reason}"),
        }
    }
}

impl ContentRenderer for HtmlRenderer<'_> {
    #[allow(clippy::expect_used)]
    fn render_steps(&self, before_steps: &str, steps: &[(String, CssClasses)]) -> Result<Content> {
        let mut result = self.render_markdown(before_steps)?;

        for (index, (step, classes)) in steps.iter().enumerate() {
            let class_str = if classes.is_empty() {
                String::new()
            } else {
                format!(" {}", classes.join(" "))
            };

            let step_html = self.render_markdown(step)?;

            write!(
                result,
                r#"
<div class="step step-{index}{class_str}"><!-- begin step -->
{step_html}</div><!-- end step -->
"#,
            )
            .expect("Writing to string should never fail");
        }

        let alt = generate_alt_text(before_steps, steps);
        Ok(Content::Html {
            raw: result,
            style: self.style.clone(),
            alt: Some(alt),
        })
    }
}

fn generate_alt_text(before_steps: &str, steps: &[(String, CssClasses)]) -> String {
    let mut result = String::new();
    result.push_str(before_steps);

    for (step, _) in steps {
        result.push('\n');
        result.push_str(step);
    }

    result
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use comrak::Options;
    use comrak::options::Plugins;
    use toboggan_core::Style;

    use super::*;

    fn setup_test_renderer() -> HtmlRenderer<'static> {
        let options = Box::leak(Box::new(Options::default()));
        let plugins = Box::leak(Box::new(Plugins::default()));
        HtmlRenderer::new(options, plugins, Style::default(), "test.md")
    }

    fn math_renderer() -> HtmlRenderer<'static> {
        let options = Box::leak(Box::new(super::super::default_options()));
        let plugins = Box::leak(Box::new(Plugins::default()));
        HtmlRenderer::new(options, plugins, Style::default(), "math.md")
    }

    fn raw_of(content: Content) -> String {
        match content {
            Content::Html { raw, .. } => raw,
            other => panic!("Expected HTML content, got {other:?}"),
        }
    }

    #[test]
    fn test_render_steps() {
        let renderer = setup_test_renderer();
        let steps = vec![
            ("First step".to_owned(), vec![]),
            ("Second step".to_owned(), vec!["highlight".to_owned()]),
        ];

        let content = renderer
            .render_steps("Before steps", &steps)
            .expect("render");

        let raw = raw_of(content);
        assert!(raw.contains("step-0"));
        assert!(raw.contains("step-1"));
        assert!(raw.contains("highlight"));
        assert!(raw.contains("First step"));
        assert!(raw.contains("Second step"));
    }

    /// `$…$` and `$$…$$` become `MathML` in the built HTML, not a `data-math-style`
    /// span waiting on a client-side renderer.
    #[test]
    fn math_is_converted_to_mathml_at_build_time() {
        let renderer = math_renderer();
        let content = renderer
            .render_steps("Inline $x^2$ and display $$y=mx+b$$", &[])
            .expect("render");

        let raw = raw_of(content);
        assert!(
            raw.contains("<msup><mi>x</mi><mn>2</mn></msup>"),
            "inline math not converted: {raw}"
        );
        assert!(
            raw.contains(r#"<math display="block">"#),
            "display math not converted: {raw}"
        );
        assert!(
            !raw.contains("data-math-style"),
            "raw LaTeX left for a client-side renderer: {raw}"
        );
    }

    /// A typo in an expression has to stop the build. Rendering it as an empty
    /// span would only be discovered in front of an audience.
    #[test]
    fn invalid_math_fails_the_build_and_names_the_file() {
        let renderer = math_renderer();
        let error = renderer
            .render_steps(r"Broken $\this_is_not_a_command$", &[])
            .expect_err("invalid LaTeX should be rejected");

        match error {
            TobogganCliError::InvalidMath { file, latex, .. } => {
                assert_eq!(file, "math.md");
                assert!(latex.contains(r"\this"), "latex not reported: {latex}");
            }
            other => panic!("Expected InvalidMath, got {other:?}"),
        }
    }

    /// Math inside a step, not just before the steps, is converted too.
    #[test]
    fn math_inside_a_step_is_converted() {
        let renderer = math_renderer();
        let content = renderer
            .render_steps("Intro", &[(r"Then $\frac{a}{b}$".to_owned(), vec![])])
            .expect("render");

        let raw = raw_of(content);
        assert!(raw.contains("<mfrac>"), "step math not converted: {raw}");
    }
}
