//! Parser module for different input formats

mod flat;
mod folder;
mod utils;

use anyhow::Result;
pub use flat::FlatFileParser;
pub use folder::FolderParser;
use toboggan_core::{Content, Date, Talk};

/// Strategy pattern for parsing different input formats into Talk structures
pub trait Parser {
    /// Parse input into a Talk structure
    fn parse(&self, title_override: Option<Content>, date_override: Option<Date>) -> Result<Talk>;
}
