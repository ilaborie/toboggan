use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use toboggan_cli::OutputFormat;
use toboggan_client::TobogganConfig;
use toboggan_core::Date;

const DEFAULT_THEME: &str = "base16-ocean.light";
const DEFAULT_WPM: u16 = 150;
const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const DEFAULT_PORT: u16 = 8080;

/// Unified Toboggan command.
///
/// With no subcommand, builds the given presentation folder in-memory and serves
/// it (watching the folder for changes) — the everyday "present this" workflow.
#[derive(Debug, Parser)]
#[command(
    name = "toboggan",
    version,
    about = "Build, serve, lint, and author Toboggan presentations",
    args_conflicts_with_subcommands = true
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,

    /// Default action arguments (used when no subcommand is given).
    #[command(flatten)]
    pub(crate) default: DefaultArgs,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Build a presentation folder into an output file (toml/json/yaml/html/typst)
    Build(BuildArgs),
    /// Serve a prebuilt talk `.toml` file
    Serve(ServeArgs),
    /// Build a folder and serve it, watching for changes (the default action)
    Watch(DefaultArgs),
    /// Scaffold a new presentation
    New(NewArgs),
    /// Run the terminal UI client against a running server
    Tui(ClientArgs),
    /// Run the desktop client against a running server
    Desktop(ClientArgs),
    /// Lint a presentation folder
    Lint(LintArgs),
    /// Print presentation statistics
    Stats(StatsArgs),
    /// Emit the bundled `OpenAPI` document
    Openapi(OpenapiArgs),
    /// Build a PDF from a folder (requires the `typst` binary)
    Pdf(PdfArgs),
    /// Generate per-slide thumbnails and an overview page
    Thumbnails(ThumbnailsArgs),
    /// Run the MCP authoring server, or install its client config
    Mcp(McpArgs),
}

/// Build options shared by `build`, the default action, `stats`, `pdf`, ...
#[derive(Debug, Clone, Args)]
pub(crate) struct BuildOptions {
    /// Override the presentation title
    #[arg(short, long)]
    pub(crate) title: Option<String>,

    /// Override the presentation date (YYYY-MM-DD)
    #[arg(short, long, value_parser = parse_date)]
    pub(crate) date: Option<Date>,

    /// Syntax-highlighting theme for code blocks
    #[arg(long, default_value = DEFAULT_THEME)]
    pub(crate) theme: String,

    /// Disable automatic numbering of parts and slides
    #[arg(long)]
    pub(crate) no_counter: bool,

    /// Speaking rate in words per minute (duration estimates)
    #[arg(long, default_value_t = DEFAULT_WPM)]
    pub(crate) wpm: u16,

    /// Exclude speaker notes from duration calculations
    #[arg(long)]
    pub(crate) exclude_notes_from_duration: bool,
}

impl BuildOptions {
    /// Builds a [`toboggan_cli::Settings`] for `input`, suppressing the stats
    /// output by default (the server/serve flow does not want it on stdout).
    pub(crate) fn into_cli_settings(
        self,
        input: PathBuf,
        no_stats: bool,
    ) -> toboggan_cli::Settings {
        toboggan_cli::Settings {
            output: None,
            title: self.title,
            date: self.date,
            theme: self.theme,
            list_themes: false,
            format: None,
            no_counter: self.no_counter,
            no_stats,
            wpm: self.wpm,
            exclude_notes_from_duration: self.exclude_notes_from_duration,
            input: Some(input),
        }
    }
}

/// Server options shared by `serve` and the default action.
#[derive(Debug, Clone, Args)]
pub(crate) struct ServeOptions {
    /// Host to bind to
    #[arg(long, env = "TOBOGGAN_HOST", default_value_t = DEFAULT_HOST)]
    pub(crate) host: IpAddr,

    /// Port to bind to
    #[arg(long, env = "TOBOGGAN_PORT", default_value_t = DEFAULT_PORT)]
    pub(crate) port: u16,

    /// Maximum number of concurrent WebSocket clients
    #[arg(long, env = "TOBOGGAN_MAX_CLIENTS", default_value_t = 100)]
    pub(crate) max_clients: usize,

    /// Allowed CORS origins (comma-separated)
    #[arg(long, env = "TOBOGGAN_CORS_ORIGINS", value_delimiter = ',')]
    pub(crate) allowed_origins: Option<Vec<String>>,

    /// Local public folder for presentation assets (served at /public/)
    #[arg(long, env = "TOBOGGAN_PUBLIC_DIR")]
    pub(crate) public_dir: Option<PathBuf>,

    /// Directory of generated thumbnails + overview.html (served at /overview/)
    #[arg(long, env = "TOBOGGAN_THUMBNAILS_DIR")]
    pub(crate) thumbnails_dir: Option<PathBuf>,

    /// Shell to spawn for embedded terminals
    #[arg(long, env = "TOBOGGAN_SHELL")]
    pub(crate) shell: Option<String>,
}

impl ServeOptions {
    fn into_server_settings(self, talk: PathBuf, watch: bool) -> toboggan_server::Settings {
        toboggan_server::Settings {
            host: self.host,
            port: self.port,
            talk,
            max_clients: self.max_clients,
            heartbeat_interval_secs: 30,
            shutdown_timeout_secs: 30,
            cleanup_interval_secs: 60,
            allowed_origins: self.allowed_origins,
            public_dir: self.public_dir,
            thumbnails_dir: self.thumbnails_dir,
            watch,
            shell: self.shell,
        }
    }
}

/// Default action: build a folder in-memory and serve it.
#[derive(Debug, Clone, Args)]
pub(crate) struct DefaultArgs {
    /// Presentation folder to build and serve
    pub(crate) input: Option<PathBuf>,

    #[command(flatten)]
    pub(crate) build: BuildOptions,

    #[command(flatten)]
    pub(crate) serve: ServeOptions,

    /// Do not watch the folder for changes
    #[arg(long)]
    pub(crate) no_watch: bool,
}

impl DefaultArgs {
    /// Returns the input folder, the CLI parse settings, the server settings, and
    /// whether to watch the folder.
    ///
    /// # Errors
    /// Returns an error if no input folder was given or it is not a directory.
    pub(crate) fn resolve(
        self,
    ) -> anyhow::Result<(
        PathBuf,
        toboggan_cli::Settings,
        toboggan_server::Settings,
        bool,
    )> {
        let input = self.input.ok_or_else(|| {
            anyhow::anyhow!(
                "no presentation folder given; pass a folder or a subcommand (try --help)"
            )
        })?;
        if !input.is_dir() {
            anyhow::bail!("input is not a directory: {}", input.display());
        }
        let watch = !self.no_watch;
        let cli_settings = self.build.into_cli_settings(input.clone(), true);
        // The in-memory serve does not read `talk` from disk; use the folder as a placeholder.
        let server_settings = self.serve.into_server_settings(input.clone(), false);
        Ok((input, cli_settings, server_settings, watch))
    }
}

#[derive(Debug, Args)]
pub(crate) struct BuildArgs {
    /// Input folder to process
    pub(crate) input: PathBuf,

    /// Output file (default: stdout). Extension drives the format.
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,

    /// Output format (auto-detected from the output extension when omitted)
    #[arg(short = 'f', long, value_enum)]
    pub(crate) format: Option<OutputFormat>,

    /// Disable the statistics output
    #[arg(long)]
    pub(crate) no_stats: bool,

    #[command(flatten)]
    pub(crate) build: BuildOptions,
}

impl From<BuildArgs> for toboggan_cli::Settings {
    fn from(args: BuildArgs) -> Self {
        let mut settings = args.build.into_cli_settings(args.input, args.no_stats);
        settings.output = args.output;
        settings.format = args.format;
        settings
    }
}

#[derive(Debug, Args)]
pub(crate) struct ServeArgs {
    /// Talk `.toml` file to serve
    pub(crate) talk: PathBuf,

    /// Watch the talk file and reload on change
    #[arg(long)]
    pub(crate) watch: bool,

    #[command(flatten)]
    pub(crate) serve: ServeOptions,
}

impl From<ServeArgs> for toboggan_server::Settings {
    fn from(args: ServeArgs) -> Self {
        args.serve.into_server_settings(args.talk, args.watch)
    }
}

#[derive(Debug, Args)]
pub(crate) struct NewArgs {
    /// Directory to create the presentation in
    pub(crate) dir: PathBuf,

    /// Presentation title (defaults to the directory name)
    #[arg(short, long)]
    pub(crate) title: Option<String>,

    /// Presentation date (YYYY-MM-DD, defaults to today)
    #[arg(short, long, value_parser = parse_date)]
    pub(crate) date: Option<Date>,

    /// Version control to initialize
    #[arg(long, value_enum, default_value_t = Vcs::Jj)]
    pub(crate) vcs: Vcs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Vcs {
    /// Jujutsu (`jj git init`)
    Jj,
    /// Git (`git init`)
    Git,
    /// No version control
    None,
}

#[derive(Debug, Args)]
pub(crate) struct ClientArgs {
    /// Server host to connect to
    #[arg(long, default_value = "localhost")]
    pub(crate) host: String,

    /// Server port to connect to
    #[arg(long, default_value_t = DEFAULT_PORT)]
    pub(crate) port: u16,
}

impl From<ClientArgs> for TobogganConfig {
    fn from(args: ClientArgs) -> Self {
        TobogganConfig::new(&args.host, args.port)
    }
}

#[derive(Debug, Args)]
pub(crate) struct StatsArgs {
    /// Input folder to analyze
    pub(crate) input: PathBuf,

    #[command(flatten)]
    pub(crate) build: BuildOptions,
}

impl From<StatsArgs> for toboggan_cli::Settings {
    fn from(args: StatsArgs) -> Self {
        args.build.into_cli_settings(args.input, false)
    }
}

#[derive(Debug, Args)]
pub(crate) struct OpenapiArgs {
    /// Output file (default: stdout)
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct PdfArgs {
    /// Input folder to process
    pub(crate) input: PathBuf,

    /// Output PDF path (default: <folder>.pdf)
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,

    #[command(flatten)]
    pub(crate) build: BuildOptions,
}

impl PdfArgs {
    pub(crate) fn cli_settings(&self) -> toboggan_cli::Settings {
        self.build
            .clone()
            .into_cli_settings(self.input.clone(), true)
    }
}

#[derive(Debug, Args)]
pub(crate) struct LintArgs {
    /// Input folder to lint
    pub(crate) input: PathBuf,

    /// Severity at or above which lint exits non-zero
    #[arg(long, value_enum, default_value_t = DenyLevel::Error)]
    pub(crate) deny: DenyLevel,

    /// Output the report as JSON
    #[arg(long)]
    pub(crate) json: bool,

    /// Also run spell checking via the `typos` CLI
    #[arg(long)]
    pub(crate) spell: bool,

    #[command(flatten)]
    pub(crate) build: BuildOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum DenyLevel {
    /// Exit non-zero on any info, warning, or error
    Info,
    /// Exit non-zero on any warning or error
    Warning,
    /// Exit non-zero only on errors
    Error,
}

#[derive(Debug, Args)]
pub(crate) struct ThumbnailsArgs {
    /// Input folder to process
    pub(crate) input: PathBuf,

    /// Output directory for thumbnails and the overview page (default: ./overview)
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,

    /// Do not emit a search index / search box
    #[arg(long)]
    pub(crate) no_search: bool,

    #[command(flatten)]
    pub(crate) build: BuildOptions,
}

#[derive(Debug, Args)]
pub(crate) struct McpArgs {
    #[command(subcommand)]
    pub(crate) action: Option<McpAction>,

    /// Presentation directory the MCP server operates on
    #[arg(long, global = true)]
    pub(crate) dir: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum McpAction {
    /// Serve the MCP authoring server over stdio (default)
    Serve,
    /// Install the MCP server config for an LLM client
    Init {
        /// Target client
        #[arg(value_enum, default_value_t = McpClient::ClaudeCode)]
        client: McpClient,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum McpClient {
    /// Claude Code
    ClaudeCode,
}

fn parse_date(input: &str) -> Result<Date, String> {
    input
        .parse::<Date>()
        .map_err(|_| format!("invalid date '{input}', expected YYYY-MM-DD"))
}
