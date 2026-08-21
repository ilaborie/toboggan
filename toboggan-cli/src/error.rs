use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use miette::{Diagnostic, NamedSource, SourceSpan};

pub type Result<T> = std::result::Result<T, TobogganCliError>;

#[derive(Debug, derive_more::Display, derive_more::Error, Diagnostic)]
pub enum TobogganCliError {
    #[display("Failed to read directory: {}", path.display())]
    #[diagnostic(
        code(toboggan_cli::read_directory),
        help("Ensure the directory exists and you have read permissions")
    )]
    ReadDirectory { path: PathBuf, source: io::Error },

    #[display("Failed to read file: {}", path.display())]
    #[diagnostic(
        code(toboggan_cli::read_file),
        help("Ensure the file exists and you have read permissions")
    )]
    ReadFile { path: PathBuf, source: io::Error },

    /// Keeps the individual failures rather than only their rendered text.
    ///
    /// `#[related]` makes miette draw each one with its own snippet and caret,
    /// which is the whole reason the variants below carry a span: the folder
    /// build collects per-slide failures here, so without this every diagnostic
    /// they produce would be flattened to a line of text and the snippet lost.
    #[display("{} slide(s) failed to parse", failures.len())]
    #[diagnostic(
        code(toboggan_cli::slides_failed_to_parse),
        help("Fix the slides above, or mark them `skip = true` to exclude them deliberately")
    )]
    SlidesFailedToParse {
        #[related]
        failures: Vec<TobogganCliError>,
    },

    /// Names the slide as well as the embed target.
    ///
    /// This is one of the two variants whose `Display` mentions only the file
    /// it *failed on*, not the file that asked for it — so a deck with several
    /// `<!-- code:… -->` directives reported a bad embed and left the author to
    /// find which slide held it.
    #[display("Invalid code embed `{}` in {slide}: {reason}", path.display())]
    #[diagnostic(
        code(toboggan_cli::invalid_code_embed),
        help(
            "`<!-- code:lang:path -->` paths are relative to the deck root and must stay inside it"
        )
    )]
    InvalidCodeEmbed {
        slide: String,
        path: PathBuf,
        reason: String,
    },

    #[display("Unknown syntax highlighting theme: {theme}")]
    #[diagnostic(
        code(toboggan_cli::unknown_theme),
        help("Run `toboggan build --list-themes` for the themes that exist")
    )]
    UnknownTheme { theme: String },

    #[display("Failed to write to standard output")]
    #[diagnostic(
        code(toboggan_cli::write_stdout),
        help(
            "Usually a broken pipe: the output was piped into a command that exited first, \
             such as `toboggan stats | head`"
        )
    )]
    WriteStdout { source: io::Error },

    #[display("Failed to create file: {}", path.display())]
    #[diagnostic(
        code(toboggan_cli::create_file),
        help("Ensure you have write permissions in the target directory")
    )]
    CreateFile { path: PathBuf, source: io::Error },

    #[display("Failed to write to file: {}", path.display())]
    #[diagnostic(
        code(toboggan_cli::write_file),
        help("Ensure you have write permissions and sufficient disk space")
    )]
    WriteFile { path: PathBuf, source: io::Error },

    #[display("Failed to parse markdown file: {}", src.name())]
    #[diagnostic(
        code(toboggan_cli::parse_markdown),
        help(
            "Check your markdown syntax. Common issues: unclosed code blocks, invalid frontmatter"
        )
    )]
    ParseMarkdown {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("error occurred here")]
        span: SourceSpan,
        message: String,
    },

    /// Carries `toml`'s own message rather than only naming the file.
    ///
    /// The span and `source_code` give miette a snippet, but most callers only
    /// ever see `Display` — the folder parser collects per-file failures into
    /// strings — and "failed to parse frontmatter in 1-hello.md" does not tell
    /// an author which key is wrong. `message()` is the reason without toml's
    /// own snippet, which miette already draws.
    #[display("Failed to parse frontmatter in file {}: {}", src.name(), source.message())]
    #[diagnostic(
        code(toboggan_cli::parse_frontmatter),
        help("Frontmatter must be valid TOML format between '+++' markers")
    )]
    ParseFrontmatter {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("{}", source.message())]
        span: SourceSpan,

        /// Not the error's `source()`: `toml::de::Error`'s own `Display` is a
        /// full report — "TOML parse error at line 5, column 1", the offending
        /// line, and a caret — so exposing it as a cause made miette print the
        /// same snippet twice and the same message three times, in two
        /// coordinate systems. Kept as data for `message()`, which is the
        /// reason on its own.
        #[error(not(source))]
        source: Box<toml::de::Error>,
    },

    #[display("Failed to format markdown file: {}", src.name())]
    #[diagnostic(
        code(toboggan_cli::format_commonmark),
        help("The markdown content could not be formatted. This might indicate corrupted AST")
    )]
    FormatCommonmark {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("formatting failed here")]
        span: SourceSpan,
        message: String,
    },

    /// The offending expression is the diagnostic's own source.
    ///
    /// The renderer re-parses each block on its own, so no offset into the
    /// containing file is available here. `src` therefore holds the expression
    /// and is *named* after the file, and `span` is relative to the expression
    /// — which is what miette draws, and where the caret belongs anyway.
    #[display("Invalid LaTeX math in {}: {reason}", src.name())]
    #[diagnostic(
        code(toboggan_cli::invalid_math),
        help(
            "`$…$` and `$$…$$` are converted to MathML while the deck builds, so a bad \
             expression is caught here rather than silently rendering as nothing in \
             front of an audience. Fix the expression, or escape the dollar sign (`\\$`) \
             if it was not meant to be math."
        )
    )]
    InvalidMath {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("{reason}")]
        span: SourceSpan,
        reason: String,
    },

    /// Carries the diagram itself, spanned. See [`Self::InvalidMath`] for why
    /// the source is the snippet rather than the file it came from.
    #[display("Invalid Mermaid diagram in {}: {reason}", src.name())]
    #[diagnostic(
        code(toboggan_cli::invalid_mermaid),
        help(
            "` ```mermaid ` fences are drawn while the deck builds, so a broken diagram is \
             caught here rather than rendering as nothing in front of an audience. Fix the \
             diagram, or fence it as ` ```text ` if it was meant to stay source."
        )
    )]
    InvalidMermaid {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("{reason}")]
        span: SourceSpan,
        reason: String,
    },

    /// Separate from [`Self::InvalidMermaid`] so the advice can be, too.
    ///
    /// A rejected parameter is the common case — a typo beats a malformed
    /// diagram in practice — and telling its author to "fix the diagram, or
    /// fence it as ` ```text `" points at a diagram that is fine and suggests
    /// throwing it away.
    #[display("Invalid Mermaid fence in {}: {reason}", src.name())]
    #[diagnostic(
        code(toboggan_cli::invalid_mermaid_fence),
        help(
            "` ```mermaid:key=value,… ` accepts theme, background, width, nodeSpacing, \
             rankSpacing, maxLabelWidth, aspectRatio, fastText, class and alt. Every \
             value with a fixed set of spellings is checked, so a misspelled one is \
             reported here too; `class` and `alt` are free text. `class` styles the \
             HTML only. Put `alt` last — its label runs to the end of the fence, so a \
             comma in it is part of the sentence rather than the next parameter."
        )
    )]
    InvalidMermaidFence {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("{reason}")]
        span: SourceSpan,
        reason: String,
    },

    #[display("Invalid Mermaid config {}: {message}", path.display())]
    #[diagnostic(
        code(toboggan_cli::mermaid_config),
        help(
            "`--mermaid-config` / `[build] mermaid-config` points at a JSON file in Mermaid's \
             own config shape: `theme`, `themeVariables`, `preferredAspectRatio`, `flowchart`"
        )
    )]
    MermaidConfig { path: PathBuf, message: String },

    #[display("Invalid date format: '{input}'")]
    #[diagnostic(
        code(toboggan_cli::invalid_date_format),
        help("Date must be in YYYY-MM-DD format (e.g., 2024-01-15)")
    )]
    InvalidDateFormat { input: String },

    #[display("Input path is not a directory: {}", path.display())]
    #[diagnostic(
        code(toboggan_cli::not_a_directory),
        help(
            "toboggan-cli only processes folder structures.\n\n\
             Please organize your presentation in a folder with the following structure:\n\n\
             my-talk/\n\
             ├── _cover.md             # Cover slide with title and date in frontmatter\n\
             ├── 01-intro/             # Section folder\n\
             │   ├── _part.md          # Section divider\n\
             │   └── slides.md         # Content slides\n\
             └── conclusion.md         # Final slide\n\n\
             Use frontmatter in _cover.md for title and date:\n\
             +++\n\
             title = \"My Presentation\"\n\
             date = \"2024-03-15\"\n\
             +++"
        )
    )]
    NotADirectory { path: PathBuf },

    #[display("No title found for presentation")]
    #[diagnostic(
        code(toboggan_cli::missing_title),
        help("Add a title in the frontmatter, first heading, or use --title flag")
    )]
    MissingTitle,

    #[display("Failed to serialize to {format}")]
    #[diagnostic(
        code(toboggan_cli::serialize),
        help("The presentation structure could not be converted to {format} format")
    )]
    Serialize { format: String, message: String },

    #[display("Failed to parse command-line arguments")]
    #[diagnostic(
        code(toboggan_cli::cli_parse),
        help("Run with --help to see available options")
    )]
    CliParse { source: clap::Error },

    #[display("Typst rendering failed: {message}")]
    #[diagnostic(
        code(toboggan_cli::typst),
        help("Ensure the `typst` binary is installed and on PATH")
    )]
    Typst { message: String },

    #[display("Failed to scaffold presentation: {message}")]
    #[diagnostic(
        code(toboggan_cli::scaffold),
        help("Choose an empty or non-existent target directory")
    )]
    Scaffold { message: String },
}

impl TobogganCliError {
    #[must_use]
    pub fn parse_markdown(
        file_path: &str,
        content: String,
        span: SourceSpan,
        message: String,
    ) -> Self {
        Self::ParseMarkdown {
            src: Arc::new(NamedSource::new(file_path, content)),
            span,
            message,
        }
    }

    #[must_use]
    pub fn parse_frontmatter(
        file_path: &str,
        content: String,
        span: SourceSpan,
        source: toml::de::Error,
    ) -> Self {
        let source = Box::new(source);
        Self::ParseFrontmatter {
            src: Arc::new(NamedSource::new(file_path, content)),
            span,
            source,
        }
    }

    /// Spans the whole expression: the `MathML` converter reports a reason, not
    /// a position.
    #[must_use]
    pub fn invalid_math(file: &str, latex: String, reason: String) -> Self {
        let span = (0, latex.len()).into();
        Self::InvalidMath {
            src: Arc::new(NamedSource::new(snippet_name(file, "math"), latex)),
            span,
            reason,
        }
    }

    /// Spans the offending parameter within the fence's info string, so the
    /// caret lands on `width=200px` rather than on the whole fence.
    #[must_use]
    pub fn invalid_mermaid_fence(
        file: &str,
        params: String,
        span: SourceSpan,
        reason: String,
    ) -> Self {
        Self::InvalidMermaidFence {
            src: Arc::new(NamedSource::new(
                snippet_name(file, "mermaid fence"),
                params,
            )),
            span,
            reason,
        }
    }

    #[must_use]
    pub fn invalid_mermaid(file: &str, diagram: String, span: SourceSpan, reason: String) -> Self {
        Self::InvalidMermaid {
            src: Arc::new(NamedSource::new(
                snippet_name(file, "mermaid fence"),
                diagram,
            )),
            span,
            reason,
        }
    }

    #[must_use]
    pub fn format_commonmark(
        file_path: &str,
        content: String,
        span: SourceSpan,
        message: String,
    ) -> Self {
        Self::FormatCommonmark {
            src: Arc::new(NamedSource::new(file_path, content)),
            span,
            message,
        }
    }
}

impl TobogganCliError {
    #[must_use]
    pub fn read_directory(path: PathBuf, source: io::Error) -> Self {
        Self::ReadDirectory { path, source }
    }

    #[must_use]
    pub fn read_file(path: PathBuf, source: io::Error) -> Self {
        Self::ReadFile { path, source }
    }

    #[must_use]
    pub fn create_file(path: PathBuf, source: io::Error) -> Self {
        Self::CreateFile { path, source }
    }

    #[must_use]
    pub fn write_file(path: PathBuf, source: io::Error) -> Self {
        Self::WriteFile { path, source }
    }

    #[must_use]
    pub fn write_stdout(source: io::Error) -> Self {
        Self::WriteStdout { source }
    }

    #[must_use]
    pub fn scaffold(message: String) -> Self {
        Self::Scaffold { message }
    }

    #[must_use]
    pub fn typst(source: &io::Error) -> Self {
        Self::Typst {
            message: format!("could not run `typst`: {source}"),
        }
    }

    #[must_use]
    pub fn typst_failed(status: &str) -> Self {
        Self::Typst {
            message: format!("`typst compile` failed: {status}"),
        }
    }
}

impl From<clap::Error> for TobogganCliError {
    fn from(source: clap::Error) -> Self {
        Self::CliParse { source }
    }
}

impl From<toml::ser::Error> for TobogganCliError {
    fn from(source: toml::ser::Error) -> Self {
        Self::Serialize {
            format: "TOML".to_owned(),
            message: source.to_string(),
        }
    }
}

impl From<serde_json::Error> for TobogganCliError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialize {
            format: "JSON".to_owned(),
            message: source.to_string(),
        }
    }
}

impl From<serde_saphyr::Error> for TobogganCliError {
    fn from(source: serde_saphyr::Error) -> Self {
        Self::Serialize {
            format: "YAML".to_owned(),
            message: source.to_string(),
        }
    }
}

/// Names a snippet so miette's header cannot be read as a file location.
///
/// These diagnostics hold the *expression* or the *diagram*, not the file it
/// came from — the renderer re-parses each block on its own, so no offset into
/// the containing file survives to rebase the span with. Naming the source after
/// the file alone made miette print `slide.md:3:3` for a caret three lines into
/// a fence that starts at line 12, sending both a reader and an editor's
/// jump-to-error to the wrong place.
fn snippet_name(file: &str, kind: &str) -> String {
    format!("{file} ({kind})")
}
