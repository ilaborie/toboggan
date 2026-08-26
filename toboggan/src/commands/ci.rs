//! Generation of the CI workflow that publishes a deck.
//!
//! Every piece needed to publish a deck already exists — the composite action in
//! this repository's `action.yml`, and a workflow that drives it — but the
//! author had to find the example, copy it and edit it by hand. That copy is
//! also where the pinned action version went stale: it is written once and never
//! revisited. Generating the file instead reads the version from this crate, so
//! the pin cannot drift from the binary that wrote it, and works out the deck's
//! path relative to the repository root, which is the part a hand-copied example
//! most often gets wrong.

use std::fs;
use std::path::Path;

use tracing::warn;

use crate::cli::{CiArgs, CiProvider};
use crate::config;

const GITHUB_PAGES_TEMPLATE: &str = include_str!("../templates/github-pages.yml");

/// Where a generated GitHub Actions workflow belongs, relative to the repo root.
const GITHUB_PAGES_PATH: &str = ".github/workflows/pages.yml";

/// The release tag of `ilaborie/toboggan` the generated workflow pins.
///
/// This crate's own version, because releases are tag-driven: `release.yml`
/// triggers on `tags: ["v*"]` and the workspace version is bumped to match the
/// tag in the same `chore(release)` commit. The pin is therefore correct except
/// in the few minutes between that bump and the tag being pushed, when it names
/// a release that does not exist yet.
pub(crate) const ACTION_TAG: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// The values the workflow template has holes for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Workflow {
    /// The deck's slides folder as the action's `folder` input — relative to the
    /// repository root, which is where the workflow file itself lives.
    folder: String,

    /// The deck's `[build] base-url`, when it has one.
    ///
    /// Usually `None`: `toboggan-cli`'s asset rewriting makes every deck asset
    /// relative to `index.html`, which is already correct for a project page
    /// under a sub-path as well as for a user page at the site root.
    base_url: Option<String>,
}

impl Workflow {
    /// The workflow a deck sitting at its repository's root generates.
    ///
    /// This is the shape the committed `examples/github-pages/pages.yml`
    /// documents, and a test holds that file to it.
    #[cfg(test)]
    pub(crate) fn example() -> Self {
        Self {
            folder: "./slides".to_owned(),
            base_url: None,
        }
    }

    /// Fills the template in.
    pub(crate) fn render(&self) -> String {
        // `base-url` hangs off the end of the `out-dir` line rather than having a
        // line of its own in the template: a placeholder on its own line leaves
        // a stray blank line behind in the common case, where it expands to
        // nothing.
        let base_url = self
            .base_url
            .as_ref()
            .map_or_else(String::new, |url| format!("\n          base-url: {url}"));
        GITHUB_PAGES_TEMPLATE
            .replace("{{version}}", ACTION_TAG)
            .replace("{{folder}}", &self.folder)
            .replace("{{base_url}}", &base_url)
    }
}

/// Writes (or prints) the CI workflow that builds and publishes a deck.
///
/// # Errors
/// Returns an error if the workflow directory or file cannot be written.
#[allow(clippy::print_stdout)]
pub(crate) fn generate(args: CiArgs, config: &config::Config) -> anyhow::Result<()> {
    // One provider today. Binding it irrefutably rather than ignoring it is what
    // makes adding a second variant a compile error here, where it needs its own
    // template and its own default path.
    let CiProvider::GithubPages = args.provider;

    let deck = super::deck::resolve_deck(&args.path.resolve(config));
    let root = super::repo_root(&deck.slides, &super::VCS_MARKERS);

    // Without a repository there is no root to make the deck path relative to
    // and nothing for Actions to run against, so fall back to the deck itself
    // and say why the result may need editing.
    let root = root.unwrap_or_else(|| {
        warn!(
            "no `.jj` or `.git` above {} — writing the workflow beside the deck; \
             move it to your repository root and adjust `folder:`",
            deck.slides.display()
        );
        deck.root()
    });

    let workflow = Workflow {
        folder: action_folder(&deck.slides, &root),
        base_url: config.build.base_url.clone(),
    };
    let rendered = workflow.render();

    if args.stdout {
        // `print!`, not `println!`: the template already ends in a newline, and
        // the extra one would make `toboggan ci --stdout > pages.yml` differ by
        // a trailing blank line from what the same command writes to a file.
        print!("{rendered}");
        return Ok(());
    }

    let path = args.output.unwrap_or_else(|| root.join(GITHUB_PAGES_PATH));

    // Never silently replace an edited workflow, for the same reason
    // `toboggan skills` does not: the file is meant to be customised, and
    // re-running the command (or `toboggan new --ci` in an existing directory)
    // would otherwise discard that work with no warning.
    if path.exists() && !args.force {
        println!(
            "↩︎ {} already exists, leaving it alone (pass --force to overwrite)",
            path.display()
        );
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, &rendered)?;

    println!("✅ Wrote {}", path.display());
    println!(
        "   deck: {} · action: ilaborie/toboggan@{ACTION_TAG}",
        workflow.folder
    );
    println!("   Enable Pages → Settings → Pages → Source: GitHub Actions");
    Ok(())
}

/// The `folder` input: `slides` as the action will address it from `root`.
///
/// Prefixed with `./` when it is a plain relative path, matching how the action
/// documents its own default and how a hand-written workflow reads. Falls back
/// to the path as given when it does not sit under `root` at all, which only
/// happens with an explicit `--output` somewhere unrelated.
fn action_folder(slides: &Path, root: &Path) -> String {
    // `slides` comes from the command line and may be relative while `root` is
    // canonicalized, so canonicalize both before comparing them.
    let canonical = slides.canonicalize();
    let absolute = canonical.as_deref().unwrap_or(slides);
    let relative = absolute.strip_prefix(root).unwrap_or(slides);
    let display = relative.display().to_string();
    if display.is_empty() {
        ".".to_owned()
    } else if display.starts_with('.') || display.starts_with('/') {
        display
    } else {
        format!("./{display}")
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// Docs that pin `ilaborie/toboggan@<tag>` and must name the current one.
    ///
    /// Paths relative to the workspace root, which is this crate's manifest
    /// directory's parent.
    const PINNING_DOCS: [&str; 5] = [
        "README.md",
        "action.yml",
        "examples/README.md",
        "examples/github-pages/pages.yml",
        "examples/toboggan-guide/slides/6_workflow/4-publish.md",
    ];

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the toboggan crate has a workspace root above it")
            .to_path_buf()
    }

    /// `examples/github-pages/pages.yml` is documentation people copy, and the
    /// version in it went stale exactly once already. It is now generator
    /// output, so it cannot: regenerate it if this fails, do not hand-edit it.
    #[test]
    fn the_committed_example_matches_what_we_generate() {
        let path = workspace_root().join("examples/github-pages/pages.yml");
        let committed = fs::read_to_string(&path).expect("the committed example must exist");
        assert_eq!(
            Workflow::example().render(),
            committed,
            "{} is out of date — regenerate it with `toboggan ci --stdout > {}`",
            path.display(),
            path.display()
        );
    }

    /// Every doc that names a version of the action names *this* one.
    ///
    /// The tag lives in five files by hand, and keeping them in step was a
    /// manual chore that had already been forgotten once. Failing here at the
    /// version bump is the whole point; the fix is to update the files it names.
    #[test]
    fn every_doc_pins_the_current_action_version() {
        let root = workspace_root();
        for doc in PINNING_DOCS {
            let text = fs::read_to_string(root.join(doc)).expect("doc must exist");
            for (occurrence, rest) in text.split("ilaborie/toboggan@").skip(1).enumerate() {
                let tag = rest
                    .chars()
                    .take_while(|char| char.is_ascii_alphanumeric() || *char == '.')
                    .collect::<String>();
                assert_eq!(
                    tag, ACTION_TAG,
                    "{doc} pins ilaborie/toboggan@{tag} at occurrence {occurrence}, \
                     but this crate is {ACTION_TAG}"
                );
            }
        }
    }

    /// `action.yml`'s `version` default is the tag the action installs when a
    /// caller does not say. It is not written as `ilaborie/toboggan@…`, so the
    /// test above cannot see it.
    #[test]
    fn the_action_installs_the_current_version_by_default() {
        let text =
            fs::read_to_string(workspace_root().join("action.yml")).expect("action.yml must exist");
        assert!(
            text.contains(&format!("default: {ACTION_TAG}")),
            "action.yml's `version` input should default to {ACTION_TAG}"
        );
    }

    /// A deck in a subdirectory publishes from a workflow at the repository
    /// root, so `folder:` has to be the path from there — the thing a copied
    /// example gets wrong, since its default assumes the deck *is* the repo.
    #[test]
    fn a_nested_deck_gets_a_repo_relative_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().canonicalize().expect("canonicalize");
        let slides = root.join("talks/keynote/slides");
        fs::create_dir_all(&slides).expect("create slides");
        fs::create_dir_all(root.join(".git")).expect("create .git");

        assert_eq!(action_folder(&slides, &root), "./talks/keynote/slides");
    }

    /// The everyday case: the deck is the repository.
    #[test]
    fn a_deck_at_the_repo_root_gets_the_default_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().canonicalize().expect("canonicalize");
        let slides = root.join("slides");
        fs::create_dir_all(&slides).expect("create slides");

        assert_eq!(action_folder(&slides, &root), "./slides");
        assert_eq!(Workflow::example().folder, action_folder(&slides, &root));
    }

    /// A `[build] base-url` reaches the action; the usual `None` leaves no trace
    /// — and in particular no blank line where the input would have gone.
    #[test]
    fn base_url_is_emitted_only_when_set() {
        let plain = Workflow::example().render();
        assert!(!plain.contains("base-url"));
        assert!(!plain.contains("out-dir: dist\n\n\n"));

        let with_base = Workflow {
            folder: "./slides".to_owned(),
            base_url: Some("/my-talk/".to_owned()),
        }
        .render();
        assert!(with_base.contains("\n          base-url: /my-talk/\n"));
    }

    /// Nothing may be left unsubstituted, mirroring the scaffold's own guard.
    ///
    /// A bare `contains("{{")` will not do here: a workflow is full of legitimate
    /// `${{ github.ref }}` expressions. Ours are the `{{…}}` that no `$` precedes.
    #[test]
    fn the_template_has_no_leftover_placeholders() {
        let rendered = Workflow::example().render();
        let leftovers = rendered
            .match_indices("{{")
            .filter(|(at, _)| *at == 0 || !rendered[..*at].ends_with('$'))
            .count();
        assert_eq!(leftovers, 0, "unsubstituted placeholder in:\n{rendered}");
    }

    /// `--stdout` uses `print!` on the strength of this.
    #[test]
    fn the_rendered_workflow_ends_in_exactly_one_newline() {
        let rendered = Workflow::example().render();
        assert!(rendered.ends_with('\n'));
        assert!(!rendered.ends_with("\n\n"));
    }
}
