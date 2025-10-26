use std::fmt::Write;

use comrak::options::Plugins;
use comrak::{Options, markdown_to_html_with_plugins};
use toboggan_core::{Content, Style};

use super::CssClasses;

pub(super) trait ContentRenderer {
    fn render_steps(&self, before_steps: &str, steps: &[(String, CssClasses)]) -> Content;

    fn render_cells(&self, cells: &[(String, CssClasses)]) -> Content;
}

pub(super) struct HtmlRenderer<'a> {
    options: &'a Options<'a>,
    plugins: &'a Plugins<'a>,
    style: Style,
}

impl<'a> HtmlRenderer<'a> {
    #[must_use]
    pub(super) fn new(options: &'a Options, plugins: &'a Plugins, style: Style) -> Self {
        Self {
            options,
            plugins,
            style,
        }
    }
}

impl ContentRenderer for HtmlRenderer<'_> {
    #[allow(clippy::expect_used)]
    fn render_steps(&self, before_steps: &str, steps: &[(String, CssClasses)]) -> Content {
        let mut result = String::new();

        let begin = markdown_to_html_with_plugins(before_steps, self.options, self.plugins);
        result.push_str(&begin);

        for (index, (step, classes)) in steps.iter().enumerate() {
            let class_str = if classes.is_empty() {
                String::new()
            } else {
                format!(" {}", classes.join(" "))
            };

            let step_html = markdown_to_html_with_plugins(step, self.options, self.plugins);

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
        Content::Html {
            raw: result,
            style: self.style.clone(),
            alt: Some(alt),
        }
    }

    #[allow(clippy::expect_used)]
    fn render_cells(&self, cells: &[(String, CssClasses)]) -> Content {
        let mut result = String::new();

        for (index, (cell_content, classes)) in cells.iter().enumerate() {
            let class_str = if classes.is_empty() {
                String::new()
            } else {
                format!(" {}", classes.join(" "))
            };

            // Render cell content with steps if it has them
            let cell_html = markdown_to_html_with_plugins(cell_content, self.options, self.plugins);

            write!(
                result,
                r#"
<div class="cell cell-{index}{class_str}"><!-- begin cell -->
{cell_html}</div><!-- end cell -->
"#,
            )
            .expect("Writing to string should never fail");
        }

        let alt = cells
            .iter()
            .map(|(content, _)| content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        Content::Html {
            raw: result,
            style: self.style.clone(),
            alt: Some(alt),
        }
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
mod tests {
    use comrak::Options;
    use comrak::options::Plugins;
    use toboggan_core::Style;

    use super::*;

    fn setup_test_renderer() -> HtmlRenderer<'static> {
        let options = Box::leak(Box::new(Options::default()));
        let plugins = Box::leak(Box::new(Plugins::default()));
        HtmlRenderer::new(options, plugins, Style::default())
    }

    #[test]
    fn test_render_steps() {
        let renderer = setup_test_renderer();
        let steps = vec![
            ("First step".to_string(), vec![]),
            ("Second step".to_string(), vec!["highlight".to_string()]),
        ];

        let content = renderer.render_steps("Before steps", &steps);

        if let Content::Html { raw, .. } = content {
            assert!(raw.contains("step-0"));
            assert!(raw.contains("step-1"));
            assert!(raw.contains("highlight"));
            assert!(raw.contains("First step"));
            assert!(raw.contains("Second step"));
        } else {
            panic!("Expected HTML content");
        }
    }

    #[test]
    fn test_render_cells() {
        let renderer = setup_test_renderer();
        let cells = vec![
            ("Cell 1 content".to_string(), vec![]),
            ("Cell 2 content".to_string(), vec!["special".to_string()]),
        ];

        let content = renderer.render_cells(&cells);

        if let Content::Html { raw, .. } = content {
            assert!(raw.contains("cell-0"));
            assert!(raw.contains("cell-1"));
            assert!(raw.contains("special"));
            assert!(raw.contains("Cell 1 content"));
            assert!(raw.contains("Cell 2 content"));
        } else {
            panic!("Expected HTML content");
        }
    }
}
