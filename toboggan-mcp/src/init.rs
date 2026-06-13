use std::path::Path;
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
