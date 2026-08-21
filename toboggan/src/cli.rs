use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand, ValueHint};
use clap_complete::Shell;
use serde::Deserialize;
use toboggan_cli::OutputFormat;
use toboggan_client::TobogganConfig;
use toboggan_core::{Date, Secret};

use crate::config;

const DEFAULT_THEME: &str = "base16-ocean.light";
const DEFAULT_WPM: u16 = 150;
const DEFAULT_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_MAX_CLIENTS: usize = 100;

/// Unified Toboggan command.
///
/// With no subcommand, builds the presentation folder in-memory and serves it
/// (watching for changes) — the everyday "present this" workflow. The folder is
/// `--path`, defaulting to the current directory, so a bare `toboggan` inside a
/// deck is the shortest form. A `toboggan.toml` can point `default-command`
/// somewhere other than serve.
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
    /// Install the authoring skill for an LLM client
    Skills(SkillsArgs),
    /// Generate a shell completion script (write it to your shell's completions dir)
    Completion(CompletionArgs),
}

/// The deck location, shared by every command that operates on a deck.
///
/// An option rather than a positional, because a positional shares its slot with
/// the subcommand name: at `toboggan <TAB>` a shell cannot tell whether you are
/// starting to type `build` or a folder, so it can complete neither well. As an
/// option it also carries a [`ValueHint`], which is what actually makes the
/// generated completion scripts offer directories.
#[derive(Debug, Clone, Args)]
pub(crate) struct PathArg {
    /// Presentation folder [default: the current directory]
    #[arg(short = 'p', long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub(crate) path: Option<PathBuf>,
}

impl PathArg {
    /// The deck to work on: the flag, else the config's `path`, else the cwd.
    pub(crate) fn resolve(&self, config: &config::Config) -> PathBuf {
        self.path
            .clone()
            .or_else(|| config.deck_path().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// Where to start looking for config files.
    ///
    /// The flag only — not the config's own `path`, which is what we are about
    /// to discover and would be circular.
    pub(crate) fn search_root(&self) -> PathBuf {
        self.path.clone().unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Build options shared by `build`, the default action, `stats`, `pdf`, ...
///
/// Fields a config file can supply are `Option`, with no clap `default_value`:
/// clap cannot distinguish its own default from a value the user typed, so a
/// default here would silently outrank every `toboggan.toml`. The real defaults
/// are applied in [`BuildOptions::into_cli_settings`], after the merge.
#[derive(Debug, Clone, Args)]
pub(crate) struct BuildOptions {
    /// Override the presentation title
    #[arg(short, long)]
    pub(crate) title: Option<String>,

    /// Override the presentation date (YYYY-MM-DD)
    #[arg(short, long, value_parser = parse_date)]
    pub(crate) date: Option<Date>,

    /// Deck language tag, e.g. `fr` [default: en, or the cover's frontmatter]
    #[arg(long)]
    pub(crate) lang: Option<String>,

    /// Base URL the exported HTML is served from, e.g. `/my-talk/`
    #[arg(long)]
    pub(crate) base_url: Option<String>,

    /// Syntax-highlighting theme for code blocks [default: base16-ocean.light]
    #[arg(long)]
    pub(crate) theme: Option<String>,

    /// Mermaid config JSON applied to every Mermaid fence
    ///
    /// Mermaid's own config shape: `theme`, `themeVariables`,
    /// `preferredAspectRatio`, `flowchart`. Per-fence `mermaid:key=value`
    /// parameters override it.
    #[arg(long, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    pub(crate) mermaid_config: Option<PathBuf>,

    /// Disable automatic numbering of parts and slides
    #[arg(long)]
    pub(crate) no_counter: bool,

    /// Speaking rate in words per minute (duration estimates) [default: 150]
    #[arg(long)]
    pub(crate) wpm: Option<u16>,

    /// Exclude speaker notes from duration calculations
    #[arg(long)]
    pub(crate) exclude_notes_from_duration: bool,
}

impl BuildOptions {
    /// Fills unset options from the config file's `[build]` table.
    ///
    /// Booleans merge with `||` rather than overriding: a plain `bool` cannot
    /// distinguish "not passed" from "passed as false", so a config file can
    /// turn one on but the CLI cannot turn a config-enabled one back off. This
    /// is documented in the scaffolded `toboggan.toml`.
    pub(crate) fn merge(&mut self, config: config::BuildConfig) {
        self.title = self.title.take().or(config.title);
        self.date = self.date.or(config.date);
        self.theme = self.theme.take().or(config.theme);
        self.lang = self.lang.take().or(config.lang);
        self.base_url = self.base_url.take().or(config.base_url);
        self.mermaid_config = self.mermaid_config.take().or(config.mermaid_config);
        self.wpm = self.wpm.or(config.wpm);
        self.no_counter |= config.no_counter.unwrap_or(false);
        self.exclude_notes_from_duration |= config.exclude_notes_from_duration.unwrap_or(false);
    }

    /// Builds a [`toboggan_cli::Settings`] for `input`, applying the defaults
    /// that clap no longer applies for us.
    ///
    /// `no_stats` is the caller's choice, not a default: `build` forwards its
    /// `--no-stats` flag, `stats` passes `false` because printing them is the
    /// whole point, and the serve/lint/pdf paths pass `true` because they do not
    /// want statistics on stdout.
    pub(crate) fn into_cli_settings(
        self,
        input: PathBuf,
        no_stats: bool,
    ) -> toboggan_cli::Settings {
        toboggan_cli::Settings {
            output: None,
            title: self.title,
            date: self.date,
            lang: self.lang,
            base_url: self.base_url,
            theme: self.theme.unwrap_or_else(|| DEFAULT_THEME.to_owned()),
            mermaid_config: self.mermaid_config,
            list_themes: false,
            format: None,
            no_counter: self.no_counter,
            no_stats,
            wpm: self.wpm.unwrap_or(DEFAULT_WPM),
            exclude_notes_from_duration: self.exclude_notes_from_duration,
            input: Some(input),
        }
    }
}

/// Server options shared by `serve` and the default action.
#[derive(Debug, Clone, Args)]
pub(crate) struct ServeOptions {
    /// Host to bind to [default: 127.0.0.1]
    #[arg(long, env = "TOBOGGAN_HOST")]
    pub(crate) host: Option<IpAddr>,

    /// Port to bind to [default: 8080]
    #[arg(long, env = "TOBOGGAN_PORT")]
    pub(crate) port: Option<u16>,

    /// Maximum number of concurrent WebSocket clients [default: 100]
    #[arg(long, env = "TOBOGGAN_MAX_CLIENTS")]
    pub(crate) max_clients: Option<usize>,

    /// Allowed CORS origins (comma-separated)
    #[arg(long, env = "TOBOGGAN_CORS_ORIGINS", value_delimiter = ',')]
    pub(crate) allowed_origins: Option<Vec<String>>,

    /// Local public folder for presentation assets (served at /public/)
    #[arg(long, env = "TOBOGGAN_PUBLIC_DIR", value_hint = ValueHint::DirPath)]
    pub(crate) public_dir: Option<PathBuf>,

    /// Directory of generated thumbnails + overview.html (served at /overview/)
    #[arg(long, env = "TOBOGGAN_THUMBNAILS_DIR", value_hint = ValueHint::DirPath)]
    pub(crate) thumbnails_dir: Option<PathBuf>,

    /// Shell to spawn for embedded terminals
    #[arg(long, env = "TOBOGGAN_SHELL")]
    pub(crate) shell: Option<String>,

    /// Open the presentation in the default browser once the server is ready
    #[arg(long, env = "TOBOGGAN_OPEN")]
    pub(crate) open: bool,

    /// Also open the presenter view — notes, next slide, and a timer
    ///
    /// Two windows for one talk: this one on your screen, the deck on the
    /// projector. Implies `--open`.
    #[arg(long, env = "TOBOGGAN_OPEN_PRESENTER")]
    pub(crate) open_presenter: bool,

    /// Secret that lets a client not on this machine drive the deck
    ///
    /// Only relevant with `--host` set to something reachable: a client on this
    /// machine always presents. Remote clients pass it as `?token=…`.
    #[arg(long, env = "TOBOGGAN_PRESENTER_TOKEN")]
    pub(crate) presenter_token: Option<Secret>,
}

impl ServeOptions {
    /// Fills unset options from the config file's `[serve]` table.
    ///
    /// See [`BuildOptions::merge`] for why `open` merges with `||`.
    pub(crate) fn merge(&mut self, config: config::ServeConfig) {
        self.host = self.host.or(config.host);
        self.port = self.port.or(config.port);
        self.max_clients = self.max_clients.or(config.max_clients);
        self.allowed_origins = self.allowed_origins.take().or(config.allowed_origins);
        self.public_dir = self.public_dir.take().or(config.public_dir);
        self.thumbnails_dir = self.thumbnails_dir.take().or(config.thumbnails_dir);
        self.shell = self.shell.take().or(config.shell);
        self.presenter_token = self.presenter_token.take().or(config.presenter_token);
        self.open |= config.open.unwrap_or(false);
        self.open_presenter |= config.open_presenter.unwrap_or(false);
    }

    fn into_server_settings(self) -> toboggan_server::ServerSettings {
        toboggan_server::ServerSettings {
            host: self.host.unwrap_or(DEFAULT_HOST),
            port: self.port.unwrap_or(DEFAULT_PORT),
            max_clients: self.max_clients.unwrap_or(DEFAULT_MAX_CLIENTS),
            heartbeat_interval_secs: 30,
            shutdown_timeout_secs: 30,
            cleanup_interval_secs: 60,
            allowed_origins: self.allowed_origins,
            public_dir: self.public_dir,
            thumbnails_dir: self.thumbnails_dir,
            shell: self.shell,
            open: self.open,
            open_presenter: self.open_presenter,
            presenter_token: self.presenter_token,
        }
    }
}

/// Default action: build a folder in-memory and serve it.
#[derive(Debug, Clone, Args)]
pub(crate) struct DefaultArgs {
    #[command(flatten)]
    pub(crate) path: PathArg,

    #[command(flatten)]
    pub(crate) build: BuildOptions,

    #[command(flatten)]
    pub(crate) serve: ServeOptions,

    /// Do not watch the folder for changes
    #[arg(long)]
    pub(crate) no_watch: bool,
}

/// Everything [`DefaultArgs::resolve`] works out from the default action's flags.
///
/// A named struct rather than a 4-tuple: the trailing `bool` meant "watch the
/// deck folder", which was easy to misread next to the server settings.
pub(crate) struct ResolvedDefault {
    pub(crate) input: PathBuf,
    pub(crate) cli: toboggan_cli::Settings,
    pub(crate) server: toboggan_server::ServerSettings,
    /// Whether to watch the deck folder and hot-swap the served talk.
    pub(crate) watch: bool,
}

impl DefaultArgs {
    /// Returns the input folder, the CLI parse settings, the server settings, and
    /// whether to watch the folder.
    ///
    /// # Errors
    /// Returns an error if the resolved input folder is not a directory.
    pub(crate) fn resolve(mut self, config: config::Config) -> anyhow::Result<ResolvedDefault> {
        let input = self.path.resolve(&config);
        if !input.is_dir() {
            anyhow::bail!("input is not a directory: {}", input.display());
        }
        self.build.merge(config.build);
        self.serve.merge(config.serve);
        let watch = !self.no_watch;
        let cli = self.build.into_cli_settings(input.clone(), true);
        // `ServerSettings`, not `Settings`: this path already has the talk in
        // memory, so there is no talk file and no `Settings::watch` to fill in
        // with a placeholder. Watching is driven by the `watch` field below.
        let server = self.serve.into_server_settings();
        Ok(ResolvedDefault {
            input,
            cli,
            server,
            watch,
        })
    }
}

/// Reinterpreting the bare-`toboggan` flags as another command's arguments.
///
/// Only the deck path and the build options carry over — they are exactly the
/// flags every selectable default command shares. The serve options have no
/// meaning for a non-serving default; rather than dropping them quietly,
/// [`DefaultArgs::warn_unused_serve_flags`] says so.
impl DefaultArgs {
    pub(crate) fn into_build_args(self) -> BuildArgs {
        BuildArgs {
            path: self.path,
            output: None,
            format: None,
            no_stats: false,
            list_themes: false,
            build: self.build,
        }
    }

    pub(crate) fn into_stats_args(self) -> StatsArgs {
        StatsArgs {
            path: self.path,
            build: self.build,
        }
    }

    pub(crate) fn into_lint_args(self) -> LintArgs {
        LintArgs {
            path: self.path,
            deny: None,
            format: None,
            json: false,
            no_spell: false,
            build: self.build,
        }
    }

    pub(crate) fn into_pdf_args(self) -> PdfArgs {
        PdfArgs {
            path: self.path,
            output: None,
            build: self.build,
        }
    }

    pub(crate) fn into_thumbnails_args(self) -> ThumbnailsArgs {
        ThumbnailsArgs {
            path: self.path,
            output: None,
            no_search: false,
            build: self.build,
        }
    }

    /// Reports server flags that the chosen default command cannot honour.
    ///
    /// These are `Option`, so "set" really means set — by a flag or by a
    /// `TOBOGGAN_*` environment variable, both of which the author expected to
    /// take effect.
    pub(crate) fn warn_unused_serve_flags(&self) {
        let mut unused = Vec::new();
        for (set, flag) in [
            (self.serve.host.is_some(), "--host"),
            (self.serve.port.is_some(), "--port"),
            (self.serve.max_clients.is_some(), "--max-clients"),
            (self.serve.allowed_origins.is_some(), "--allowed-origins"),
            (self.serve.public_dir.is_some(), "--public-dir"),
            (self.serve.thumbnails_dir.is_some(), "--thumbnails-dir"),
            (self.serve.shell.is_some(), "--shell"),
            (self.serve.presenter_token.is_some(), "--presenter-token"),
            (self.serve.open, "--open"),
            (self.serve.open_presenter, "--open-presenter"),
        ] {
            if set {
                unused.push(flag);
            }
        }
        if !unused.is_empty() {
            tracing::warn!(
                "ignoring {} — `default-command` in the configuration does not start a server",
                unused.join(", ")
            );
        }
    }
}

#[derive(Debug, Args)]
pub(crate) struct BuildArgs {
    #[command(flatten)]
    pub(crate) path: PathArg,

    /// Output file (required to write the deck). Extension drives the format.
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    pub(crate) output: Option<PathBuf>,

    /// Output format (auto-detected from the output extension when omitted)
    #[arg(short = 'f', long, value_enum)]
    pub(crate) format: Option<OutputFormat>,

    /// Disable the statistics output
    #[arg(long)]
    pub(crate) no_stats: bool,

    /// Print the syntax-highlighting theme names accepted by `--theme` and exit
    #[arg(long)]
    pub(crate) list_themes: bool,

    #[command(flatten)]
    pub(crate) build: BuildOptions,
}

impl BuildArgs {
    pub(crate) fn resolve(mut self, config: config::Config) -> toboggan_cli::Settings {
        let input = self.path.resolve(&config);
        self.build.merge(config.build);
        let mut settings = self.build.into_cli_settings(input, self.no_stats);
        settings.output = self.output;
        settings.format = self.format;
        // Handled before the deck is read, so it works from anywhere — which is
        // what the scaffolded `toboggan.toml` tells the user to expect.
        settings.list_themes = self.list_themes;
        settings
    }
}

#[derive(Debug, Args)]
pub(crate) struct ServeArgs {
    /// Prebuilt talk `.toml` file to serve
    ///
    /// A file, not a folder — this command serves an already-built talk. Use the
    /// default action (or `watch`) to build a slides folder and serve that.
    #[arg(
        short = 'p',
        long,
        value_name = "TALK_TOML",
        value_hint = ValueHint::FilePath
    )]
    pub(crate) path: PathBuf,

    /// Watch the talk file and reload on change
    #[arg(long)]
    pub(crate) watch: bool,

    #[command(flatten)]
    pub(crate) serve: ServeOptions,
}

impl ServeArgs {
    pub(crate) fn resolve(mut self, config: config::Config) -> toboggan_server::Settings {
        self.serve.merge(config.serve);
        toboggan_server::Settings {
            server: self.serve.into_server_settings(),
            talk: self.path,
            watch: self.watch,
        }
    }
}

#[derive(Debug, Args)]
pub(crate) struct NewArgs {
    /// Directory to create the presentation in
    ///
    /// Required, unlike every other `--path`: scaffolding into an implicit
    /// current directory is too easy to do by accident.
    #[arg(short = 'p', long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub(crate) path: PathBuf,

    /// Presentation title (defaults to the directory name)
    #[arg(short, long)]
    pub(crate) title: Option<String>,

    /// Presentation date (YYYY-MM-DD, defaults to today)
    #[arg(short, long, value_parser = parse_date)]
    pub(crate) date: Option<Date>,

    /// Version control to initialize
    #[arg(long, value_enum, default_value_t = Vcs::Jj)]
    pub(crate) vcs: Vcs,

    /// Skip writing the project-local `.mcp.json` MCP server config
    #[arg(long)]
    pub(crate) no_mcp: bool,

    /// Skip installing the Claude Code authoring skill
    #[arg(long)]
    pub(crate) no_skill: bool,
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

    /// Presenter token, when the server is on another machine
    ///
    /// Not needed for the usual `--host localhost`: a client on the server's
    /// own machine always presents. Without it, a client connecting across the
    /// network can watch but not navigate.
    #[arg(long, env = "TOBOGGAN_PRESENTER_TOKEN")]
    pub(crate) presenter_token: Option<Secret>,
}

impl From<ClientArgs> for TobogganConfig {
    fn from(args: ClientArgs) -> Self {
        TobogganConfig::new(&args.host, args.port)
            .with_presenter_token(args.presenter_token.clone())
    }
}

#[derive(Debug, Args)]
pub(crate) struct StatsArgs {
    #[command(flatten)]
    pub(crate) path: PathArg,

    #[command(flatten)]
    pub(crate) build: BuildOptions,
}

impl StatsArgs {
    pub(crate) fn resolve(mut self, config: config::Config) -> toboggan_cli::Settings {
        let input = self.path.resolve(&config);
        self.build.merge(config.build);
        self.build.into_cli_settings(input, false)
    }
}

#[derive(Debug, Args)]
pub(crate) struct OpenapiArgs {
    /// Output file (default: stdout)
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    pub(crate) output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct PdfArgs {
    #[command(flatten)]
    pub(crate) path: PathArg,

    /// Output PDF path (default: `<deck-name>.pdf` in the current directory)
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    pub(crate) output: Option<PathBuf>,

    #[command(flatten)]
    pub(crate) build: BuildOptions,
}

impl PdfArgs {
    /// Resolves the deck folder and the parse settings for it.
    pub(crate) fn resolve(mut self, config: config::Config) -> (PathBuf, toboggan_cli::Settings) {
        let input = self.path.resolve(&config);
        self.build.merge(config.build);
        let settings = self.build.into_cli_settings(input.clone(), true);
        (input, settings)
    }
}

#[derive(Debug, Args)]
pub(crate) struct LintArgs {
    #[command(flatten)]
    pub(crate) path: PathArg,

    /// Severity at or above which lint exits non-zero [default: error]
    #[arg(long, value_enum)]
    pub(crate) deny: Option<DenyLevel>,

    /// How to render the report [default: human]
    #[arg(long, value_enum)]
    pub(crate) format: Option<LintFormat>,

    /// Output the report as JSON (shorthand for `--format json`)
    #[arg(long, conflicts_with = "format")]
    pub(crate) json: bool,

    /// Skip spell checking (runs by default via the `typos` CLI when available)
    #[arg(long)]
    pub(crate) no_spell: bool,

    #[command(flatten)]
    pub(crate) build: BuildOptions,
}

/// How `toboggan lint` renders its report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LintFormat {
    /// Coloured lines for a terminal
    #[default]
    Human,
    /// The `LintReport` as JSON
    Json,
    /// GitHub Actions workflow commands, which become inline PR annotations
    Github,
    /// SARIF 2.1.0, for GitHub code scanning and other analysis tools
    Sarif,
}

/// Everything `lint` needs once the flags and the config file are merged.
pub(crate) struct ResolvedLint {
    pub(crate) input: PathBuf,
    pub(crate) settings: toboggan_cli::Settings,
    pub(crate) deny: DenyLevel,
    pub(crate) format: LintFormat,
    pub(crate) lint: toboggan_lint::LintConfig,
}

impl LintArgs {
    pub(crate) fn resolve(mut self, config: config::Config) -> ResolvedLint {
        let input = self.path.resolve(&config);
        self.build.merge(config.build);
        let settings = self.build.into_cli_settings(input.clone(), true);

        let file = config.lint;
        let mut lint = toboggan_lint::LintConfig::default();
        if let Some(max) = file.max_steps_per_slide {
            lint.max_steps_per_slide = max;
        }
        if let Some(max) = file.max_words_per_slide {
            lint.max_words_per_slide = max;
        }
        if let Some(max) = file.max_images_per_slide {
            lint.max_images_per_slide = max;
        }
        if let Some(max) = file.max_code_lines {
            lint.max_code_lines = max;
        }
        // Both stay `None`/`false` unless the config asks: the two rules they
        // enable are silent by default, because no deck has a natural length
        // and plenty deliberately carry no notes.
        lint.max_duration = file.max_duration;
        lint.require_notes = file.require_notes.unwrap_or(false);
        lint.disabled.extend(file.disabled.unwrap_or_default());
        lint.severity_overrides
            .extend(file.severity.unwrap_or_default());
        // The spell rule is disabled by id, so an unknown id here would be
        // reported by the linter like any other — see `toboggan_lint::rules::ids`.
        if self.no_spell || file.no_spell.unwrap_or(false) {
            lint.disable(toboggan_lint::rules::ids::SPELLING_TYPO);
        }

        // `--json` predates `--format` and still works. clap rejects passing
        // both, so the precedence here only ever picks one of them.
        let format = if self.json {
            Some(LintFormat::Json)
        } else {
            self.format
        };

        ResolvedLint {
            input,
            settings,
            deny: self.deny.or(file.deny).unwrap_or(DenyLevel::Error),
            format: format.or(file.format).unwrap_or_default(),
            lint,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
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
    #[command(flatten)]
    pub(crate) path: PathArg,

    /// Output directory for thumbnails and the overview page (default: ./overview)
    #[arg(short, long, value_hint = ValueHint::DirPath)]
    pub(crate) output: Option<PathBuf>,

    /// Do not emit a search index / search box
    #[arg(long)]
    pub(crate) no_search: bool,

    #[command(flatten)]
    pub(crate) build: BuildOptions,
}

impl ThumbnailsArgs {
    /// Resolves the deck folder and the parse settings for it.
    pub(crate) fn resolve(mut self, config: config::Config) -> (PathBuf, toboggan_cli::Settings) {
        let input = self.path.resolve(&config);
        self.build.merge(config.build);
        let settings = self.build.into_cli_settings(input.clone(), true);
        (input, settings)
    }
}

#[derive(Debug, Args)]
pub(crate) struct McpArgs {
    #[command(subcommand)]
    pub(crate) action: Option<McpAction>,

    /// Presentation directory the MCP server operates on [default: the current directory]
    #[arg(short = 'p', long, global = true, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub(crate) path: Option<PathBuf>,
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

#[derive(Debug, Args)]
pub(crate) struct SkillsArgs {
    /// Target LLM client
    #[arg(long, value_enum, default_value_t = McpClient::ClaudeCode)]
    pub(crate) target: McpClient,

    /// Directory to install the skill into [default: the current directory]
    #[arg(short = 'p', long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub(crate) path: Option<PathBuf>,

    /// Overwrite an existing SKILL.md
    #[arg(long)]
    pub(crate) force: bool,
}

#[derive(Debug, Args)]
pub(crate) struct CompletionArgs {
    /// Shell to generate the completion script for
    #[arg(value_enum)]
    pub(crate) shell: Shell,
}

fn parse_date(input: &str) -> Result<Date, String> {
    input
        .parse::<Date>()
        .map_err(|_| format!("invalid date '{input}', expected YYYY-MM-DD"))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use clap::CommandFactory as _;

    use super::*;

    /// Clap validates its argument graph at *runtime*, and `Cli::command()` is on
    /// the live path for `--help` and `toboggan completion`. A duplicate short
    /// flag or colliding id across the flattened `BuildOptions`/`ServeOptions`
    /// would therefore panic for users rather than fail the build; this covers
    /// every subcommand and flag definition in one assertion.
    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    /// `toboggan mcp init` and `toboggan new` both register this binary with
    /// Claude Code, and neither can start a server this CLI refuses to parse.
    /// They emitted `--dir` for a flag named `--path`, so every scaffolded deck
    /// shipped an MCP server that died at argument parsing — invisible from
    /// this side, because nothing ever ran the argv it wrote.
    #[test]
    fn mcp_registration_args_parse() {
        let registered = std::iter::once("toboggan")
            .chain(toboggan_mcp::SERVER_ARGS)
            .chain(std::iter::once("/tmp/deck"));

        let cli = Cli::try_parse_from(registered).expect("the argv we register must parse");
        let Some(Commands::Mcp(mcp)) = cli.command else {
            panic!("expected the mcp subcommand, got {:?}", cli.command);
        };
        assert_eq!(mcp.path.as_deref(), Some(Path::new("/tmp/deck")));
    }
}
