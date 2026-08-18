use std::path::{Path, PathBuf};
use std::process::Command;

/// The argv prefix this binary is registered with, before the deck directory.
///
/// Shared by all three registration paths — `claude mcp add`, the printed
/// fallback, and `.mcp.json` — because they disagreed once: they emitted
/// `--dir` after the flag was renamed to `--path`, so every scaffolded deck
/// shipped an MCP server that failed at argument parsing. `toboggan`'s
/// `mcp_registration_args_parse` test drives this through the real CLI.
pub const SERVER_ARGS: [&str; 2] = ["mcp", "--path"];

/// Registers this binary as the `toboggan` MCP server for Claude Code.
///
/// Prefers `claude mcp add`, falling back to printing the configuration to add
/// manually. Registration is delegated entirely to the `claude` CLI, so whether
/// an existing `toboggan` entry is replaced is that tool's decision, not this
/// function's.
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
        .args(SERVER_ARGS)
        .arg(&root)
        .status();

    // Split the two failures: "the CLI is missing" and "the CLI ran and refused"
    // need different things from the reader, and the most likely refusal is a
    // `toboggan` server that is already registered.
    match status {
        Ok(status) if status.success() => {
            println!("✅ Registered the `toboggan` MCP server with Claude Code");
        }
        Ok(status) => {
            println!("`claude mcp add` exited with {status} (is `toboggan` already registered?).");
            print_manual_config(&exe, &root);
        }
        Err(err) => {
            println!("Could not run `claude mcp add` ({err}).");
            print_manual_config(&exe, &root);
        }
    }
    Ok(())
}

#[allow(clippy::print_stdout)]
fn print_manual_config(exe: &Path, root: &Path) {
    println!("Add the server to your Claude Code MCP config:");
    println!("  command: {}", exe.display());
    let [command, flag] = SERVER_ARGS;
    println!(
        "  args:    [\"{command}\", \"{flag}\", \"{}\"]",
        root.display()
    );
}

/// Writes a project-local `.mcp.json` in `project` registering this binary as
/// the `toboggan` MCP server, scoped to `slides`.
///
/// The two paths differ: the config belongs at the deck root (where the editor
/// looks for it), while the server's root must be the *slides* folder. Pointing
/// the server at the deck root made `talk_outline` return an empty deck and
/// `add_slide` write orphan files next to `slides/`, because the parser treats
/// its path directly as the slides folder with no `slides/` descent.
///
/// Self-contained — unlike [`mcp_init`] it never shells out to the `claude` CLI,
/// so it works in any environment. Used by `toboggan new` to wire up authoring
/// out of the box.
///
/// An existing `.mcp.json` is merged rather than replaced: overwriting one would
/// silently drop every other MCP server the author had configured.
///
/// # Errors
/// Returns an error if the current executable path cannot be determined, an
/// existing `.mcp.json` is present but unreadable or not valid JSON, or the file
/// cannot be written.
pub fn write_mcp_json(project: &Path, slides: &Path) -> anyhow::Result<PathBuf> {
    let exe = std::env::current_exe()?.to_string_lossy().into_owned();
    let dir = slides
        .canonicalize()
        .unwrap_or_else(|_| slides.to_path_buf())
        .to_string_lossy()
        .into_owned();
    let path = project.join(".mcp.json");

    let mut config = match std::fs::read_to_string(&path) {
        Ok(existing) => serde_json::from_str::<serde_json::Value>(&existing).map_err(|err| {
            anyhow::anyhow!("{} exists but is not valid JSON: {err}", path.display())
        })?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(err) => anyhow::bail!("reading {}: {err}", path.display()),
    };

    let servers = config
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("{} is not a JSON object", path.display()))?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));
    let servers = servers
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("{}: `mcpServers` is not an object", path.display()))?;
    servers.insert(
        "toboggan".to_owned(),
        serde_json::json!({
            "command": exe,
            "args": [SERVER_ARGS[0], SERVER_ARGS[1], dir],
        }),
    );

    let body = serde_json::to_string_pretty(&config)?;
    std::fs::write(&path, format!("{body}\n"))?;
    Ok(path)
}
