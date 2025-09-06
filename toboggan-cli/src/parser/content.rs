use std::path::Path;
use std::string::ToString;

use comrak::nodes::NodeValue;
use comrak::{Options, Plugins, format_commonmark};
use miette::SourceSpan;
use toboggan_core::{Content, Slide, SlideKind};

use crate::error::{Result, TobogganCliError};
use crate::parser::comments::{is_notes, parse_cell, parse_code, parse_pause};
use crate::parser::directory::{extract_node_text, parse_frontmatter};
use crate::parser::{
    ContentRenderer, CssClasses, DEFAULT_SLIDE_TITLE, FrontMatter, HtmlRenderer, MarkdownNode,
    default_options,
};

#[derive(Debug, Clone, Default)]
pub struct InnerContent {
    before_steps: String,
    steps: Vec<(String, CssClasses)>,
    next_step: Option<(String, CssClasses)>,
}

impl InnerContent {
    fn handle<'a>(&mut self, elt: &'a MarkdownNode<'a>, file_name: &str) -> Result<()> {
        // Check if this is a code comment and handle it differently
        let mut code_block_md = None;
        {
            let data = elt.data.borrow();
            if let NodeValue::HtmlBlock(html) = &data.value
                && let Some((info, path)) = parse_code(&html.literal)
            {
                // Instead of modifying the AST, generate markdown directly
                let file_content = std::fs::read_to_string(&path)
                    .map_err(|source| TobogganCliError::read_file(path.clone(), source))?;

                // Generate a fenced code block markdown
                code_block_md = Some(format!("```{info}\n{file_content}\n```\n"));
            }
        }

        let md = if let Some(code_md) = code_block_md {
            // Use the generated code block markdown
            code_md
        } else {
            // Regular processing
            let data = &elt.data.borrow().value;
            let mut buffer = String::new();
            let options = default_options();
            format_commonmark(elt, &options, &mut buffer).map_err(|source| {
                TobogganCliError::FormatCommonmark {
                    src: miette::NamedSource::new(file_name, format!("{data:?}")),
                    span: SourceSpan::from((0, 1)),
                    message: source.to_string(),
                }
            })?;
            let mut md = buffer;

            if let NodeValue::HtmlBlock(html) = data {
                // Detect new pause
                if let Some(classes) = parse_pause(&html.literal) {
                    if let Some(current) = self.next_step.take() {
                        self.steps.push(current);
                    }
                    self.next_step = Some((String::default(), classes));
                    md = String::new();
                }
            }

            md
        };

        if let Some((next, _)) = &mut self.next_step {
            next.push_str(&md);
        } else {
            self.before_steps.push_str(&md);
        }

        Ok(())
    }

    fn render_with<R: ContentRenderer>(&self, renderer: &R) -> Content {
        let all_steps: Vec<_> = self
            .steps
            .iter()
            .chain(self.next_step.iter())
            .cloned()
            .collect();
        renderer.render_steps(&self.before_steps, &all_steps)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CellsContent {
    cells: Vec<(InnerContent, CssClasses)>,
    next_cell: Option<(InnerContent, CssClasses)>,
}

impl CellsContent {
    fn handle<'a>(&mut self, elt: &'a MarkdownNode<'a>, file_name: &str) -> Result<()> {
        let data = &elt.data.borrow().value;
        if let NodeValue::HtmlBlock(html) = data {
            // Detect new cell
            if let Some(classes) = parse_cell(&html.literal) {
                if let Some(current) = self.next_cell.take() {
                    self.cells.push(current);
                }
                self.next_cell = Some((InnerContent::default(), classes));
            }
        }

        if let Some((next, _)) = &mut self.next_cell {
            next.handle(elt, file_name)?;
        } else {
            let mut next = InnerContent::default();
            next.handle(elt, file_name)?;
            self.next_cell = Some((next, CssClasses::default()));
        }

        Ok(())
    }

    fn render_with<R: ContentRenderer>(&self, renderer: &R) -> Content {
        let cell_contents: Vec<_> = self
            .cells
            .iter()
            .chain(self.next_cell.iter())
            .map(|(inner_content, classes)| {
                // For now, we'll extract the content as markdown
                // In a full refactor, we'd have the renderer handle InnerContent directly
                let mut content = inner_content.before_steps.clone();
                for (step, _) in &inner_content.steps {
                    content.push('\n');
                    content.push_str(step);
                }
                if let Some((next_step, _)) = &inner_content.next_step {
                    content.push('\n');
                    content.push_str(next_step);
                }
                (content, classes.clone())
            })
            .collect();

        renderer.render_cells(&cell_contents)
    }
}

#[derive(Debug, Clone)]
pub enum SlideContentParser {
    Init,
    Base {
        fm: FrontMatter,
        title: Option<String>,
        inner: InnerContent,
    },
    Grid {
        fm: FrontMatter,
        title: Option<String>,
        cells: CellsContent,
    },
    Notes {
        fm: FrontMatter,
        title: Option<String>,
        inner: InnerContent,
        notes: InnerContent,
    },
    GridNotes {
        fm: FrontMatter,
        title: Option<String>,
        cells: CellsContent,
        notes: InnerContent,
    },
}

impl SlideContentParser {
    #[must_use]
    pub fn new() -> Self {
        Self::Init
    }

    /// Handle frontmatter parsing and state transitions
    fn handle_frontmatter(&mut self, content: &str, file_name: &str) -> Result<()> {
        let frontmatter = parse_frontmatter(content, file_name)?;

        if frontmatter.grid {
            *self = Self::Grid {
                fm: frontmatter,
                title: None,
                cells: CellsContent::default(),
            };
        } else if let Self::Base { fm, .. } = self {
            *fm = frontmatter;
        }
        Ok(())
    }

    /// Handle heading nodes and extract titles
    fn handle_heading(&mut self, heading_text: String) {
        match self {
            Self::Base { title, .. } | Self::Grid { title, .. } => {
                if title.is_none() && !heading_text.is_empty() {
                    *title = Some(heading_text);
                }
            }
            _ => {}
        }
    }

    /// Transition to notes state
    fn transition_to_notes(&mut self) {
        match self {
            Self::Base { fm, title, inner } => {
                *self = Self::Notes {
                    fm: fm.clone(),
                    title: title.clone(),
                    inner: inner.clone(),
                    notes: InnerContent::default(),
                };
            }
            Self::Grid { fm, title, cells } => {
                *self = Self::GridNotes {
                    fm: fm.clone(),
                    title: title.clone(),
                    cells: cells.clone(),
                    notes: InnerContent::default(),
                };
            }
            _ => {}
        }
    }

    fn handle<'a>(&mut self, elt: &'a MarkdownNode<'a>, file_name: &str) -> Result<()> {
        let data = &elt.data.borrow().value;
        match self {
            Self::Init => {
                *self = Self::Base {
                    fm: FrontMatter::default(),
                    title: None,
                    inner: InnerContent::default(),
                };
                self.handle(elt, file_name)?;
            }
            Self::Base { inner, .. } => match data {
                NodeValue::FrontMatter(content) => {
                    self.handle_frontmatter(content, file_name)?;
                }
                NodeValue::Heading(_) => {
                    let heading_text = extract_node_text(elt);
                    self.handle_heading(heading_text);
                    // Need to access inner after self is borrowed mutably
                    if let Self::Base { inner, .. } = self {
                        inner.handle(elt, file_name)?;
                    }
                }
                NodeValue::HtmlBlock(html) if is_notes(&html.literal) => {
                    self.transition_to_notes();
                }
                _ => inner.handle(elt, file_name)?,
            },
            Self::Grid { fm, title, cells } => match data {
                NodeValue::HtmlBlock(html) if is_notes(&html.literal) => {
                    let new_state = Self::GridNotes {
                        fm: fm.clone(),
                        title: title.clone(),
                        cells: cells.clone(),
                        notes: InnerContent::default(),
                    };
                    *self = new_state;
                }
                NodeValue::Heading(_) if title.is_none() => {
                    *title = Some(extract_node_text(elt));
                    cells.handle(elt, file_name)?;
                }
                _ => cells.handle(elt, file_name)?,
            },
            Self::Notes { notes, .. } | Self::GridNotes { notes, .. } => {
                notes.handle(elt, file_name)?;
            }
        }

        Ok(())
    }

    fn front_matter(&self) -> FrontMatter {
        match self {
            Self::Init => FrontMatter::default(),
            Self::Base { fm, .. }
            | Self::Grid { fm, .. }
            | Self::Notes { fm, .. }
            | Self::GridNotes { fm, .. } => fm.clone(),
        }
    }

    fn title(&self) -> Option<String> {
        match self {
            Self::Init => None,
            Self::Base { fm, title, .. }
            | Self::Grid { fm, title, .. }
            | Self::Notes { fm, title, .. }
            | Self::GridNotes { fm, title, .. } => fm.title.clone().or_else(|| title.clone()),
        }
    }

    fn notes(&self, renderer: &HtmlRenderer) -> Content {
        match self {
            Self::Init | Self::Base { .. } | Self::Grid { .. } => Content::Empty,
            Self::Notes { notes, .. } | Self::GridNotes { notes, .. } => {
                notes.render_with(renderer)
            }
        }
    }

    fn body(&self, renderer: &HtmlRenderer) -> Content {
        match self {
            Self::Init => Content::Empty,
            Self::Base { inner, .. } | Self::Notes { inner, .. } => inner.render_with(renderer),
            Self::Grid { cells, .. } | Self::GridNotes { cells, .. } => cells.render_with(renderer),
        }
    }

    pub fn parse<'a, I>(
        mut self,
        iterator: I,
        options: &Options,
        plugins: &Plugins,
        name: Option<&str>,
        path: Option<&Path>,
    ) -> Result<(Slide, FrontMatter)>
    where
        I: Iterator<Item = &'a MarkdownNode<'a>>,
    {
        let file_name = path.map_or_else(
            || "<unknown>".to_string(),
            |path| path.to_string_lossy().to_string(),
        );

        for elt in iterator {
            self.handle(elt, &file_name)?;
        }

        let front_matter = self.front_matter();
        let style = front_matter.to_style()?;
        let renderer = HtmlRenderer::new(options, plugins, style.clone());

        let result = Slide {
            kind: SlideKind::Standard,
            style,
            title: Content::Text {
                text: self
                    .title()
                    .or_else(|| name.map(ToString::to_string))
                    .unwrap_or_else(|| DEFAULT_SLIDE_TITLE.to_string()),
            },
            body: self.body(&renderer),
            notes: self.notes(&renderer),
        };

        Ok((result, front_matter))
    }

    /// Convenience method to parse with default options and plugins
    pub fn parse_with_defaults<'a, I>(
        self,
        iterator: I,
        name: Option<&str>,
        path: Option<&Path>,
    ) -> Result<(Slide, FrontMatter)>
    where
        I: Iterator<Item = &'a MarkdownNode<'a>>,
    {
        use crate::parser::{default_options, default_plugins};
        let options = default_options();
        let plugins = default_plugins();
        self.parse(iterator, &options, &plugins, name, path)
    }
}

impl Default for SlideContentParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use comrak::{Arena, parse_document};
    use toboggan_core::Content;

    use super::*;
    use crate::parser::{default_options, default_plugins};

    fn parse_markdown_content(content: &str) -> Result<(Slide, FrontMatter)> {
        let arena = Arena::new();
        let options = default_options();
        let plugins = default_plugins();

        let root = parse_document(&arena, content, &options);

        let parser = SlideContentParser::new();
        parser.parse(root.children(), &options, &plugins, None, None)
    }

    #[test]
    fn test_basic_slide_parsing() -> Result<()> {
        let markdown = "# Test Title\n\nThis is basic content.";
        let (slide, _) = parse_markdown_content(markdown)?;

        assert_eq!(slide.title.to_string(), "Test Title");
        assert!(matches!(slide.body, Content::Html { .. }));

        Ok(())
    }

    #[test]
    fn test_frontmatter_parsing() -> Result<()> {
        let markdown = r#"+++
title = "Frontmatter Title"
classes = ["custom-class"]
+++

# Main Title

Content here."#;

        let (_slide, front_matter) = parse_markdown_content(markdown)?;

        assert_eq!(front_matter.title, Some("Frontmatter Title".to_string()));
        assert_eq!(front_matter.classes, vec!["custom-class"]);
        assert!(!front_matter.grid);

        Ok(())
    }

    #[test]
    fn test_grid_parsing() -> Result<()> {
        let markdown = r"+++
grid = true
+++

# Grid Title

<!-- cell -->
First cell content

<!-- cell -->
Second cell content";

        let (slide, front_matter) = parse_markdown_content(markdown)?;

        assert!(front_matter.grid);
        if let Content::Html { raw, .. } = slide.body {
            assert!(raw.contains("cell-0"));
            assert!(raw.contains("cell-1"));
        }

        Ok(())
    }

    #[test]
    fn test_notes_parsing() -> Result<()> {
        let markdown = r"# Title

Main content here.

<!-- notes -->

These are speaker notes.";

        let (slide, _) = parse_markdown_content(markdown)?;

        assert!(matches!(slide.notes, Content::Html { .. }));
        if let Content::Html { raw, .. } = slide.notes {
            assert!(raw.contains("speaker notes"));
        }

        Ok(())
    }

    #[test]
    fn test_pause_parsing() -> Result<()> {
        let markdown = r"# Title

Before pause.

<!-- pause -->

After pause.";

        let (slide, _) = parse_markdown_content(markdown)?;

        if let Content::Html { raw, .. } = slide.body {
            assert!(raw.contains("step-0"));
            assert!(raw.contains("Before pause"));
            assert!(raw.contains("After pause"));
        }

        Ok(())
    }

    #[test]
    fn test_pause_with_classes() -> Result<()> {
        let markdown = r"# Title

Content before.

<!-- pause: highlight -->

Highlighted content.";

        let (slide, _) = parse_markdown_content(markdown)?;

        if let Content::Html { raw, .. } = slide.body {
            assert!(raw.contains("highlight"));
        }

        Ok(())
    }

    #[test]
    fn test_grid_with_notes() -> Result<()> {
        let markdown = r"+++
grid = true
+++

# Grid with Notes

<!-- cell -->
Cell 1

<!-- cell -->
Cell 2

<!-- notes -->
Grid notes here.";

        let (slide, front_matter) = parse_markdown_content(markdown)?;

        assert!(front_matter.grid);
        assert!(matches!(slide.notes, Content::Html { .. }));

        Ok(())
    }

    #[test]
    fn test_filename_as_default_title() -> Result<()> {
        let markdown = "Content without title";

        let arena = Arena::new();
        let options = default_options();
        let plugins = default_plugins();
        let root = parse_document(&arena, markdown, &options);

        let parser = SlideContentParser::new();
        let (slide, _) =
            parser.parse(root.children(), &options, &plugins, Some("my-slide"), None)?;

        assert_eq!(slide.title.to_string(), "my-slide");

        Ok(())
    }

    #[test]
    fn test_explicit_title_precedence_over_filename() -> Result<()> {
        let markdown = "# Explicit Title\n\nContent here";

        let arena = Arena::new();
        let options = default_options();
        let plugins = default_plugins();
        let root = parse_document(&arena, markdown, &options);

        let parser = SlideContentParser::new();
        let (slide, _) =
            parser.parse(root.children(), &options, &plugins, Some("filename"), None)?;

        // Explicit title should take precedence over filename
        assert_eq!(slide.title.to_string(), "Explicit Title");

        Ok(())
    }

    #[test]
    fn test_empty_content() -> Result<()> {
        let markdown = "";
        let (slide, _) = parse_markdown_content(markdown)?;

        assert_eq!(slide.title.to_string(), "<No Title>");

        Ok(())
    }

    #[test]
    fn test_title_precedence() -> Result<()> {
        let markdown = r#"+++
title = "FM Title"
+++

# Markdown Title

Content."#;

        let (slide, _) = parse_markdown_content(markdown)?;

        // Frontmatter title should take precedence
        assert_eq!(slide.title.to_string(), "FM Title");

        Ok(())
    }

    #[test]
    fn test_alt_text_generation() -> Result<()> {
        let markdown = "# Title\n\nSome **bold** text with *emphasis*.";
        let (slide, _) = parse_markdown_content(markdown)?;

        if let Content::Html { alt: Some(alt), .. } = slide.body {
            assert!(alt.contains("bold"));
            assert!(alt.contains("emphasis"));
        }

        Ok(())
    }

    #[test]
    fn test_css_style_in_markdown() -> Result<()> {
        // Test that CSS can be included as a style block in markdown
        let markdown = r#"+++
title = "CSS Style Test"
classes = ["custom", "styled"]
+++

# Test Title

<style>
body { background-color: blue; }
.custom { color: red; }
</style>

Content with inline CSS."#;

        let (slide, front_matter) = parse_markdown_content(markdown)?;

        // Check that the style contains the classes
        let style = front_matter.to_style()?;
        assert_eq!(style.classes, vec!["custom", "styled"]);

        // Check that the slide style also contains the classes
        assert_eq!(slide.style.classes, vec!["custom", "styled"]);

        // The body should contain the style block as HTML
        let body_str = slide.body.to_string();
        assert!(body_str.contains("<style>"));
        assert!(body_str.contains("background-color: blue"));

        Ok(())
    }

    #[test]
    fn test_code_comment_transformation() -> Result<()> {
        use std::fs;

        use tempfile::tempdir;

        // Create a temporary directory and file with some code
        let temp_dir = tempdir()?;
        let code_file = temp_dir.path().join("example.rs");
        let code_content = r#"fn main() {
    println!("Hello from code comment!");
}"#;
        fs::write(&code_file, code_content)?;

        // Create markdown with a code comment
        let code_path = code_file.to_string_lossy();
        let markdown = format!(
            r"# Code Comment Test

Content before code.

<!-- code:rust:{code_path} -->

Content after code."
        );

        let (slide, _) = parse_markdown_content(&markdown)?;

        // Verify the slide body contains the code as HTML
        if let Content::Html { raw, .. } = slide.body {
            // Should contain a code block with the rust language and our code content
            assert!(raw.contains("<code"));
            assert!(raw.contains("Hello from code comment!"));
            assert!(raw.contains("fn main"));
        } else {
            panic!("Expected HTML content in slide body");
        }

        Ok(())
    }

    #[test]
    fn test_code_comment_missing_file() {
        let markdown = r"# Code Comment Test

<!-- code:rust:/non/existent/file.rs -->

Content after code.";

        // Should return an error when parsing because the code file doesn't exist
        let result = parse_markdown_content(markdown);
        assert!(result.is_err());
    }

    #[test]
    fn test_code_comment_comprehensive_integration() -> Result<()> {
        use std::fs;

        use tempfile::tempdir;

        // Create temporary directory with different code files
        let temp_dir = tempdir()?;

        // Rust file
        let rust_file = temp_dir.path().join("hello.rs");
        let rust_content = r#"fn main() {
    println!("Hello, Rust!");
}"#;
        fs::write(&rust_file, rust_content)?;

        // JavaScript file
        let js_file = temp_dir.path().join("script.js");
        let js_content = r#"console.log("Hello, JavaScript!");"#;
        fs::write(&js_file, js_content)?;

        // Create comprehensive markdown with various features
        let rust_path = rust_file.to_string_lossy();
        let js_path = js_file.to_string_lossy();
        let markdown = format!(
            r"# Code Integration Test

Before any code.

<!-- code:rust:{rust_path} -->

Some explanation between code blocks.

<!-- code:javascript:{js_path} -->

## Grid Example

+++
grid = true
+++

<!-- cell -->
Cell 1 content

<!-- code:rust:{rust_path} -->

<!-- cell -->
Cell 2 with regular content

## With Pauses

Content before pause.

<!-- pause -->

<!-- code:javascript:{js_path} -->

After code with pause.

<!-- notes -->

These are speaker notes with code:

<!-- code:rust:{rust_path} -->

End of notes."
        );

        let (slide, _front_matter) = parse_markdown_content(&markdown)?;

        // Verify the slide contains all the expected code
        if let Content::Html { raw, .. } = &slide.body {
            // Should contain both Rust and JavaScript code
            assert!(raw.contains("Hello, Rust!"));
            assert!(raw.contains("Hello, JavaScript!"));
            assert!(raw.contains("fn main"));
            assert!(raw.contains("console.log"));

            // Should contain code block HTML elements
            assert!(raw.contains("<code"));

            // Should contain the explanatory text
            assert!(raw.contains("Some explanation"));
        } else {
            panic!("Expected HTML content in slide body");
        }

        // Verify notes contain the code
        if let Content::Html { raw, .. } = &slide.notes {
            assert!(raw.contains("Hello, Rust!"));
            assert!(raw.contains("speaker notes"));
        } else {
            panic!("Expected HTML content in slide notes");
        }

        Ok(())
    }
}
