use std::path::Path;
use std::process::Command;

use toboggan_core::Date;
use tracing::{info, warn};

use crate::cli::{NewArgs, Vcs};

/// Scaffolds a new presentation folder and initializes version control.
///
/// # Errors
/// Returns an error if the target exists and is non-empty, or if any file or
/// directory cannot be created.
#[allow(clippy::print_stdout)]
pub(crate) fn scaffold(args: NewArgs) -> anyhow::Result<()> {
    let dir = &args.dir;

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
    println!("   Build & serve it with:  toboggan {}", dir.display());
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
