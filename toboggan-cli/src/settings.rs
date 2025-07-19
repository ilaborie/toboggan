use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
pub struct Settings {
    /// Output file
    #[clap(short, long)]
    pub output: Option<PathBuf>,

    /// The input file to process
    pub input: PathBuf,
}
