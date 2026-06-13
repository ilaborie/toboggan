use std::fs;
use std::path::Path;
use std::process::Command;

use toboggan_core::Date;
use tracing::{info, warn};

use crate::cli::{NewArgs, Vcs};

const COVER_TEMPLATE: &str = include_str!("../templates/_cover.md");
const PART_TEMPLATE: &str = include_str!("../templates/_part.md");
const SLIDE_TEMPLATE: &str = include_str!("../templates/01-welcome.md");
const HEAD_TEMPLATE: &str = include_str!("../templates/_head.html");
const GITIGNORE_TEMPLATE: &str = include_str!("../templates/gitignore");
const MISE_TEMPLATE: &str = include_str!("../templates/mise.toml");

/// Scaffolds a new presentation folder and initializes version control.
///
/// # Errors
/// Returns an error if the target exists and is non-empty, or if any file or
/// directory cannot be created.
#[allow(clippy::print_stdout)]
pub(crate) fn scaffold(args: NewArgs) -> anyhow::Result<()> {
    let dir = &args.dir;
    if dir.is_dir() && fs::read_dir(dir)?.next().is_some() {
        anyhow::bail!(
            "directory {} already exists and is not empty",
            dir.display()
        );
    }

    let title = args.title.unwrap_or_else(|| {
        dir.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("New Talk")
            .to_owned()
    });
    let date = args.date.unwrap_or_else(Date::today);

    create_structure(dir, &title, date)?;
    init_vcs(dir, args.vcs);

    println!("✅ Created presentation \"{title}\" at {}", dir.display());
    println!("   Build & serve it with:  toboggan {}", dir.display());
    Ok(())
}

fn create_structure(dir: &Path, title: &str, date: Date) -> anyhow::Result<()> {
    let slides = dir.join("slides");
    let part = slides.join("01-introduction");
    fs::create_dir_all(&part)?;
    fs::create_dir_all(dir.join("public"))?;

    let cover = COVER_TEMPLATE
        .replace("{{title}}", title)
        .replace("{{date}}", &date.to_string());
    fs::write(slides.join("_cover.md"), cover)?;
    fs::write(slides.join("_head.html"), HEAD_TEMPLATE)?;
    fs::write(part.join("_part.md"), PART_TEMPLATE)?;
    fs::write(part.join("01-welcome.md"), SLIDE_TEMPLATE)?;
    fs::write(dir.join("public/.gitkeep"), "")?;
    fs::write(dir.join(".gitignore"), GITIGNORE_TEMPLATE)?;

    let slug = slugify(title);
    let mise = MISE_TEMPLATE.replace("{{slug}}", &slug);
    fs::write(dir.join("mise.toml"), mise)?;

    Ok(())
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
    let mut current = Some(dir);
    while let Some(path) = current {
        if path.join(marker).exists() {
            return true;
        }
        current = path.parent();
    }
    false
}

fn slugify(title: &str) -> String {
    let slug = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = slug.trim_matches('-').to_owned();
    // Collapse runs of '-'
    let mut result = String::with_capacity(trimmed.len());
    let mut last_dash = false;
    for ch in trimmed.chars() {
        if ch == '-' {
            if !last_dash {
                result.push(ch);
            }
            last_dash = true;
        } else {
            result.push(ch);
            last_dash = false;
        }
    }
    if result.is_empty() {
        "presentation".to_owned()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_handles_spaces_and_symbols() {
        assert_eq!(slugify("My Great Talk!"), "my-great-talk");
        assert_eq!(slugify("  Rust   &  WASM  "), "rust-wasm");
        assert_eq!(slugify("***"), "presentation");
    }
}
