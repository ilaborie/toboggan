//! MCP server exposing Toboggan authoring tools.
//!
//! Inspection tools (`talk_outline`, `stats`, `lint`) re-parse the presentation
//! folder, and mutation tools (`add_part`, `add_slide`) edit it through a safe
//! workspace that confines every path to the presentation root. Served over
//! stdio for LLM clients.
#![warn(missing_docs)]

mod init;
mod server;
mod workspace;

pub use self::init::{SERVER_ARGS, mcp_init, write_mcp_json};
pub use self::server::TobogganServer;

pub(crate) const ADVICE: &str = include_str!("advice.md");

use std::path::PathBuf;

use rmcp::ServiceExt;
use rmcp::transport::io::stdio;

/// Runs the MCP authoring server over stdio for the presentation folder `root`.
///
/// # Errors
/// Returns an error if the server fails to start or the session ends abnormally.
pub async fn serve_stdio(root: PathBuf) -> anyhow::Result<()> {
    let server = TobogganServer::new(root);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
