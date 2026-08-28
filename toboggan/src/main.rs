use std::future::Future;
use std::path::{Path, PathBuf};

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use toboggan_cli::TobogganCliError;
use tracing_subscriber::EnvFilter;

mod cli;
mod commands;
mod config;

use cli::{Cli, Commands, CompletionArgs, DefaultArgs, McpAction};
use config::DefaultCommand;

fn main() -> miette::Result<()> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                let _ = err.print();
                std::process::exit(0);
            }
            _ => return Err(miette::miette!(err)),
        },
    };

    init_logging(cli.command.as_ref());
    dispatch(cli)
}

/// Directives applied under whatever `RUST_LOG` says.
///
/// `chromiumoxide` logs a warning for every `DevTools` message it cannot fit into
/// its own enum, and current Chrome sends several per navigation — so
/// photographing a deck for the slide overview printed a screenful of
/// `WS Invalid message` at the author over something they can do nothing about
/// and that did not stop anything. Its real failures come back as errors from
/// the calls we make, so `error` keeps them.
const QUIET: &str = "chromiumoxide=error";

/// The env filter every non-TUI logger uses: `RUST_LOG` if set, plus [`QUIET`].
///
/// Appended rather than prepended, so a `RUST_LOG=chromiumoxide=debug` meant to
/// debug the browser still wins.
fn env_filter() -> EnvFilter {
    EnvFilter::from_default_env().add_directive(
        QUIET
            .parse()
            .unwrap_or_else(|_| tracing::level_filters::LevelFilter::INFO.into()),
    )
}

/// Initializes logging appropriately for the chosen subcommand.
///
/// The TUI needs `tui_logger` (a stdout logger would corrupt the terminal); the
/// desktop app uses a plain logger; everything else uses an env-filtered logger.
fn init_logging(command: Option<&Commands>) {
    match command {
        Some(Commands::Tui(_)) => {
            let _ = toboggan_tui::init_tui_logger();
        }
        Some(Commands::Desktop(_)) => {
            tracing_subscriber::fmt::init();
        }
        // The MCP server uses stdout for its protocol; logs must go to stderr.
        Some(Commands::Mcp(_)) => {
            tracing_subscriber::fmt()
                .with_writer(std::io::stderr)
                .with_env_filter(env_filter())
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter())
                .init();
        }
    }
}

fn dispatch(cli: Cli) -> miette::Result<()> {
    match cli.command {
        None => run_default(cli.default),
        Some(Commands::Build(args)) => {
            let config = load_config(&args.path.search_root())?;
            let settings = args.resolve(config);
            toboggan_cli::run(&settings).map_err(miette::Report::new)
        }
        Some(Commands::Serve(args)) => {
            // The talk is a file, so a config would live beside it, not "in" it.
            let root = args
                .path
                .parent()
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
            let settings = args.resolve(load_config(&root)?);
            block_on(toboggan_server::launch(settings))
        }
        Some(Commands::Watch(args)) => serve_default(args),
        Some(Commands::New(args)) => to_miette(commands::new::scaffold(args)),
        Some(Commands::Tui(args)) => block_on(toboggan_tui::run(&args.into())),
        Some(Commands::Desktop(args)) => to_miette(toboggan_desktop::run(args.into())),
        Some(Commands::Stats(args)) => stats_default(args),
        Some(Commands::Openapi(args)) => to_miette(commands::misc::emit_openapi(args)),
        Some(Commands::Pdf(args)) => pdf_default(args),
        Some(Commands::Lint(args)) => lint_default(args),
        Some(Commands::Thumbnails(args)) => thumbnails_default(args),
        Some(Commands::Mcp(args)) => {
            let dir = args.path.unwrap_or_else(|| PathBuf::from("."));
            match args.action {
                Some(McpAction::Init { client: _ }) => to_miette(toboggan_mcp::mcp_init(&dir)),
                _ => block_on(toboggan_mcp::serve_stdio(dir)),
            }
        }
        Some(Commands::Skills(args)) => to_miette(commands::skills::install(args)),
        Some(Commands::Ci(args)) => {
            let config = load_config(&args.path.search_root())?;
            to_miette(commands::ci::generate(args, &config))
        }
        Some(Commands::Completion(args)) => {
            generate_completion(&args);
            Ok(())
        }
    }
}

/// Runs the no-subcommand action, which the configuration may redirect.
///
/// `toboggan` on its own is the command people type most, so a deck can decide
/// what it should mean — `default-command = "lint"` in a CI-oriented checkout,
/// for instance. An explicit subcommand always wins over this.
fn run_default(args: DefaultArgs) -> miette::Result<()> {
    let config = load_config(&args.path.search_root())?;
    let command = config.default_command.unwrap_or_default();
    if command != DefaultCommand::Serve {
        args.warn_unused_serve_flags();
    }
    match command {
        DefaultCommand::Serve => serve_default(args),
        DefaultCommand::Build => {
            let settings = args.into_build_args().resolve(config);
            toboggan_cli::run(&settings).map_err(miette::Report::new)
        }
        DefaultCommand::Stats => to_miette(commands::misc::show_stats(
            args.into_stats_args().resolve(config),
        )),
        DefaultCommand::Lint => to_miette(commands::lint::run_lint(
            args.into_lint_args().resolve(config),
        )),
        DefaultCommand::Pdf => {
            let args = args.into_pdf_args();
            let check_overflow = !args.no_overflow_check;
            let (input, settings) = args.resolve(config);
            to_miette(commands::pdf::build_pdf(
                &input,
                settings,
                None,
                check_overflow,
            ))
        }
        DefaultCommand::Thumbnails => thumbnails_default(args.into_thumbnails_args()),
    }
}

/// Loads and merges the `toboggan.toml` layers that apply at `root`.
fn load_config(root: &Path) -> miette::Result<config::Config> {
    config::load(root).map_err(|err| miette::miette!("{err:?}"))
}

fn serve_default(args: DefaultArgs) -> miette::Result<()> {
    let config = load_config(&args.path.search_root())?;
    block_on(commands::build_serve::build_and_serve(args, config))
}

fn stats_default(args: cli::StatsArgs) -> miette::Result<()> {
    let config = load_config(&args.path.search_root())?;
    to_miette(commands::misc::show_stats(args.resolve(config)))
}

fn lint_default(args: cli::LintArgs) -> miette::Result<()> {
    let config = load_config(&args.path.search_root())?;
    to_miette(commands::lint::run_lint(args.resolve(config)))
}

fn pdf_default(args: cli::PdfArgs) -> miette::Result<()> {
    let config = load_config(&args.path.search_root())?;
    let output = args.output.clone();
    let check_overflow = !args.no_overflow_check;
    let (input, settings) = args.resolve(config);
    to_miette(commands::pdf::build_pdf(
        &input,
        settings,
        output,
        check_overflow,
    ))
}

fn thumbnails_default(mut args: cli::ThumbnailsArgs) -> miette::Result<()> {
    let config = load_config(&args.path.search_root())?;
    args.overview.merge(config.overview.clone());
    let options = commands::thumbnails::Options {
        output: args.output.clone(),
        search: !args.no_search,
        static_links: args.static_links,
        renderer: args.overview.thumbnail_renderer,
        browser: args.overview.browser.clone(),
    };
    let (input, settings) = args.resolve(config);
    // Async because the browser path serves the deck to itself over loopback
    // while it photographs it.
    block_on(commands::thumbnails::generate(&input, settings, options))
}

/// Writes a shell completion script for `toboggan` to stdout.
fn generate_completion(args: &CompletionArgs) {
    let mut command = Cli::command();
    let bin_name = command.get_name().to_owned();
    clap_complete::generate(args.shell, &mut command, bin_name, &mut std::io::stdout());
}

/// Runs an async server/client task on a dedicated multi-thread runtime.
fn block_on<F>(future: F) -> miette::Result<()>
where
    F: Future<Output = anyhow::Result<()>>,
{
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|err| miette::miette!("creating tokio runtime: {err}"))?;
    to_miette(runtime.block_on(future))
}

/// Bridges an `anyhow::Result` into a `miette::Result`.
///
/// A [`TobogganCliError`] is handed to miette whole rather than stringified:
/// it is a `Diagnostic`, so it carries the per-slide snippets, carets and help
/// text that make a parse failure findable. Flattening it here is what used to
/// leave `lint`, `pdf` and `thumbnails` printing one bare line where `build`
/// drew a caret under the offending token.
///
/// Anything else keeps the debug representation, which is what preserves an
/// `anyhow` context chain.
fn to_miette(result: anyhow::Result<()>) -> miette::Result<()> {
    result.map_err(|err| match err.downcast::<TobogganCliError>() {
        Ok(diagnostic) => miette::Report::new(diagnostic),
        Err(err) => miette::miette!("{err:?}"),
    })
}
