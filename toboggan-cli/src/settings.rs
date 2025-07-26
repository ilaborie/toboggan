use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
pub struct Settings {
    /// Output file
    #[clap(short, long)]
    pub output: Option<PathBuf>,

    /// Title for the presentation (overrides title.md/title.txt files)
    #[clap(short, long)]
    pub title: Option<String>,

    /// Date for the presentation (YYYY-MM-DD format, defaults to today)
    #[clap(short, long)]
    pub date: Option<String>,

    /// The input file or folder to process
    pub input: PathBuf,
}
