use std::path::Path;
use std::process::Command;

use toboggan_core::Date;
use tracing::{info, warn};

use crate::cli::{CiArgs, CiProvider, McpClient, NewArgs, PathArg, SkillsArgs, Vcs};

/// Scaffolds a new presentation folder and initializes version control.
///
/// # Errors
/// Returns an error if the target exists and is non-empty, or if any file or
/// directory cannot be created.
#[allow(clippy::print_stdout)]
pub(crate) fn scaffold(args: NewArgs) -> anyhow::Result<()> {
    let dir = &args.path;

    let title = args.title.unwrap_or_else(|| {
        dir.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("New Talk")
            .to_owned()
    });
    let date = args.date.unwrap_or_else(Date::today);

    toboggan_cli::scaffold::create_presentation(dir, &title, date)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    init_vcs(dir, args.vcs);

    println!("✅ Created presentation \"{title}\" at {}", dir.display());

    // Wire up Claude Code authoring by default (opt out with --no-mcp/--no-skill).
    //
    // The MCP server's root is the *slides* folder, not the deck root: pointing
    // it at the root made `talk_outline` come back empty and `add_slide` write
    // orphan files next to `slides/` where nothing would ever read them.
    if !args.no_mcp {
        match toboggan_mcp::write_mcp_json(dir, &dir.join("slides")) {
            Ok(path) => println!("✅ Wrote MCP config at {}", path.display()),
            Err(err) => warn!("could not write .mcp.json ({err}); skipping MCP setup"),
        }
    }
    if !args.no_skill {
        let skill_args = SkillsArgs {
            target: McpClient::ClaudeCode,
            path: Some(dir.clone()),
            // A freshly scaffolded directory has no SKILL.md to protect.
            force: false,
        };
        if let Err(err) = crate::commands::skills::install(skill_args) {
            warn!("could not install authoring skill ({err}); skipping");
        }
    }
    // Opt-in, and after `init_vcs`, so the repository this deck belongs to
    // already exists and the workflow lands at its root — which is the deck
    // itself for a standalone talk, and the enclosing checkout for a deck
    // scaffolded inside one.
    if args.ci {
        let ci_args = CiArgs {
            provider: CiProvider::GithubPages,
            path: PathArg {
                path: Some(dir.clone()),
            },
            output: None,
            stdout: false,
            // A freshly scaffolded directory has no workflow to protect; an
            // enclosing repository may, and `generate` leaves that one alone.
            force: false,
        };
        // The deck's own freshly written `toboggan.toml`, so a `[build]
        // base-url` set by a `--base-url` in the scaffold would reach the
        // workflow. An unreadable config is not worth failing the scaffold
        // over; the defaults are what a new deck has anyway.
        let config = crate::config::load(dir).unwrap_or_default();
        if let Err(err) = crate::commands::ci::generate(ci_args, &config) {
            warn!("could not write the CI workflow ({err}); skipping");
        }
    }

    print_next_steps(dir);
    Ok(())
}

/// Tells the author what to do with the folder that just appeared.
///
/// A scaffold that only says "created" leaves you to guess the workflow; these
/// are the four commands that cover it, in the order they are usually wanted.
/// `cd` first, because every command defaults to the current directory.
#[allow(clippy::print_stdout)]
fn print_next_steps(dir: &Path) {
    println!("\n   Next steps:");
    println!("     cd {}", dir.display());
    println!(
        "     toboggan                    # build + serve, live reload → http://localhost:8080"
    );
    println!("     toboggan lint               # check the deck");
    println!("     toboggan build -o out.html  # export a standalone page");
    println!("\n   Settings live in toboggan.toml — every option is documented there.");
}

fn init_vcs(dir: &Path, vcs: Vcs) {
    let (program, args): (&str, &[&str]) = match vcs {
        Vcs::None => return,
        Vcs::Jj => ("jj", &["git", "init"]),
        Vcs::Git => ("git", &["init"]),
    };

    // Skip if the directory is already inside a repo of the chosen kind.
    if already_in_repo(dir, vcs) {
        info!("existing {program} repository detected, skipping init");
        return;
    }

    match Command::new(program).args(args).current_dir(dir).status() {
        Ok(status) if status.success() => info!("initialized {program} repository"),
        Ok(status) => warn!("{program} {args:?} exited with {status}; skipping VCS init"),
        Err(err) => warn!("could not run `{program}` ({err}); skipping VCS init"),
    }
}

fn already_in_repo(dir: &Path, vcs: Vcs) -> bool {
    let marker = match vcs {
        Vcs::Jj => ".jj",
        Vcs::Git => ".git",
        Vcs::None => return false,
    };
    // One marker, not both: `--vcs git` inside a Jujutsu checkout should still
    // get its `git init`, and vice versa.
    super::repo_root(dir, &[marker]).is_some()
}
