use std::future::Future;

use clap::Parser;
use clap::error::ErrorKind;
use miette::IntoDiagnostic;
use tracing_subscriber::EnvFilter;

mod cli;
mod commands;

use cli::{Cli, Commands};

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
        _ => {
            tracing_subscriber::fmt()
                .with_env_filter(EnvFilter::from_default_env())
                .init();
        }
    }
}

fn dispatch(cli: Cli) -> miette::Result<()> {
    match cli.command {
        None => block_on(commands::build_serve::build_and_serve(cli.default)),
        Some(Commands::Build(args)) => {
            let settings = toboggan_cli::Settings::from(args);
            toboggan_cli::run(&settings).into_diagnostic()
        }
        Some(Commands::Serve(args)) => {
            let settings = toboggan_server::Settings::from(args);
            block_on(toboggan_server::launch(settings))
        }
        Some(Commands::Watch(args)) => block_on(commands::build_serve::build_and_serve(args)),
        Some(Commands::New(args)) => to_miette(commands::new::scaffold(args)),
        Some(Commands::Tui(args)) => block_on(toboggan_tui::run(&args.into())),
        Some(Commands::Desktop(args)) => to_miette(toboggan_desktop::run(args.into())),
        Some(Commands::Stats(args)) => to_miette(commands::misc::show_stats(args)),
        Some(Commands::Openapi(args)) => to_miette(commands::misc::emit_openapi(args)),
        Some(Commands::Pdf(args)) => to_miette(commands::pdf::build_pdf(args)),
        Some(Commands::Lint(_)) => commands::stub::coming_soon("lint"),
        Some(Commands::Thumbnails(_)) => commands::stub::coming_soon("thumbnails"),
        Some(Commands::Mcp(_)) => commands::stub::coming_soon("mcp"),
    }
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

/// Bridges an `anyhow::Result` into a `miette::Result`, preserving the context
/// chain via the debug representation.
fn to_miette(result: anyhow::Result<()>) -> miette::Result<()> {
    result.map_err(|err| miette::miette!("{err:?}"))
}
