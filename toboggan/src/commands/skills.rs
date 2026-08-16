use std::fs;
use std::path::PathBuf;

use crate::cli::SkillsArgs;

const SKILL_TEMPLATE: &str = include_str!("../templates/SKILL.md");

/// Installs the Toboggan authoring skill for an LLM client (Claude Code).
///
/// Writes `.claude/skills/toboggan-authoring/SKILL.md` plus a `.symposium`
/// marker, mirroring the in-repo skill layout.
///
/// # Errors
/// Returns an error if the skill directory or files cannot be written.
#[allow(clippy::print_stdout)]
pub(crate) fn install(args: SkillsArgs) -> anyhow::Result<()> {
    let base = args.dir.unwrap_or_else(|| PathBuf::from("."));
    let skill_dir = base.join(".claude/skills/toboggan-authoring");
    fs::create_dir_all(&skill_dir)?;

    let skill_file = skill_dir.join("SKILL.md");
    // Never silently replace an edited skill: authors tailor SKILL.md to their
    // deck, and re-running `toboggan skills` (or `toboggan new` in an existing
    // directory) used to discard that work with no warning.
    if skill_file.exists() && !args.force {
        println!(
            "↩︎ {} already exists, leaving it alone (pass --force to overwrite)",
            skill_file.display()
        );
        return Ok(());
    }
    fs::write(&skill_file, SKILL_TEMPLATE)?;
    fs::write(skill_dir.join(".symposium"), "")?;

    println!(
        "✅ Installed the toboggan-authoring skill at {}",
        skill_file.display()
    );
    Ok(())
}
