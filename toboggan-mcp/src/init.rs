use std::path::{Path, PathBuf};
use std::process::Command;

/// Registers this binary as the `toboggan` MCP server for Claude Code.
///
/// Prefers `claude mcp add`; if the `claude` CLI is unavailable, prints the
/// configuration to add manually. Never clobbers existing servers.
///
/// # Errors
/// Returns an error only if the current executable path cannot be determined.
#[allow(clippy::print_stdout)]
pub fn mcp_init(root: &Path) -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    let status = Command::new("claude")
        .args(["mcp", "add", "toboggan", "--"])
        .arg(&exe)
        .arg("mcp")
        .arg("--dir")
        .arg(&root)
        .status();

    match status {
        Ok(status) if status.success() => {
            println!("✅ Registered the `toboggan` MCP server with Claude Code");
        }
        _ => {
            println!(
                "Could not run `claude mcp add`. Add the server to your Claude Code MCP config:"
            );
            println!("  command: {}", exe.display());
            println!("  args:    [\"mcp\", \"--dir\", \"{}\"]", root.display());
        }
    }
    Ok(())
}

/// Writes a project-local `.mcp.json` in `root` registering this binary as the
/// `toboggan` MCP server, scoped to `root`.
///
/// Self-contained — unlike [`mcp_init`] it never shells out to the `claude` CLI,
/// so it works in any environment. Used by `toboggan new` to wire up authoring
/// out of the box. Overwrites any existing `.mcp.json`.
///
/// # Errors
/// Returns an error if the current executable path cannot be determined or the
/// file cannot be written.
pub fn write_mcp_json(root: &Path) -> anyhow::Result<PathBuf> {
    let exe = std::env::current_exe()?.to_string_lossy().into_owned();
    let dir = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .to_string_lossy()
        .into_owned();
    let config = serde_json::json!({
        "mcpServers": {
            "toboggan": {
                "command": exe,
                "args": ["mcp", "--dir", dir],
            }
        }
    });
    let path = root.join(".mcp.json");
    let body = serde_json::to_string_pretty(&config)?;
    std::fs::write(&path, format!("{body}\n"))?;
    Ok(path)
}
