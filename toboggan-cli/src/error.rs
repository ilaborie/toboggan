#![allow(unused_assignments)]

use std::io;
use std::path::PathBuf;

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
        src: NamedSource<String>,
        #[label("error occurred here")]
        span: SourceSpan,
        message: String,
    },

    #[display("Failed to parse frontmatter in file: {}", src.name())]
    #[diagnostic(
        code(toboggan_cli::parse_frontmatter),
        help("Frontmatter must be valid TOML format between '+++' markers")
    )]
    ParseFrontmatter {
        #[source_code]
        src: NamedSource<String>,
        #[label("invalid TOML syntax")]
        span: SourceSpan,

        source: toml::de::Error,
    },

    #[display("Failed to format markdown file: {}", src.name())]
    #[diagnostic(
        code(toboggan_cli::format_commonmark),
        help("The markdown content could not be formatted. This might indicate corrupted AST")
    )]
    FormatCommonmark {
        #[source_code]
        src: NamedSource<String>,
        #[label("formatting failed here")]
        span: SourceSpan,
        message: String,
    },

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
}

impl TobogganCliError {
    /// Helper to create a `NamedSource` with consistent naming
    fn create_named_source(file_path: &str, content: String) -> NamedSource<String> {
        NamedSource::new(file_path, content)
    }

    #[must_use]
    pub fn parse_markdown(
        file_path: &str,
        content: String,
        span: SourceSpan,
        message: String,
    ) -> Self {
        Self::ParseMarkdown {
            src: Self::create_named_source(file_path, content),
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
        Self::ParseFrontmatter {
            src: Self::create_named_source(file_path, content),
            span,
            source,
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
            src: Self::create_named_source(file_path, content),
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
}

impl From<clap::Error> for TobogganCliError {
    fn from(source: clap::Error) -> Self {
        Self::CliParse { source }
    }
}

impl From<toml::ser::Error> for TobogganCliError {
    fn from(source: toml::ser::Error) -> Self {
        Self::Serialize {
            format: "TOML".to_string(),
            message: source.to_string(),
        }
    }
}

impl From<serde_json::Error> for TobogganCliError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialize {
            format: "JSON".to_string(),
            message: source.to_string(),
        }
    }
}

impl From<serde_saphyr::Error> for TobogganCliError {
    fn from(source: serde_saphyr::Error) -> Self {
        Self::Serialize {
            format: "YAML".to_string(),
            message: source.to_string(),
        }
    }
}

impl From<std::io::Error> for TobogganCliError {
    fn from(source: std::io::Error) -> Self {
        Self::ReadFile {
            path: PathBuf::from("<unknown>"),
            source,
        }
    }
}
