use clap::Parser;
use clap::error::ErrorKind;
use miette::IntoDiagnostic;
use toboggan_cli::{Settings, run};

fn main() -> miette::Result<()> {
    tracing_subscriber::fmt().pretty().init();

    let settings = match Settings::try_parse() {
        Ok(settings) => settings,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                // Let clap handle help and version output normally
                let _ = err.print();
                std::process::exit(0);
            }
            _ => {
                // Convert other errors to miette diagnostics
                return Err(miette::miette!(err));
            }
        },
    };

    run(&settings).into_diagnostic()?;

    Ok(())
}
