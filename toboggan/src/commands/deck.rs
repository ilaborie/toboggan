//! Shared deck resolution and parsing for every subcommand.
//!
//! Two rules live here because every command has to follow them identically:
//! where the slides are, and what to do when one of them fails to parse.

use std::path::{Path, PathBuf};

use toboggan_core::Talk;

/// A deck's on-disk layout.
pub(crate) struct Deck {
    /// The folder the parser walks.
    pub(crate) slides: PathBuf,
    /// A sibling `public/` directory, when one exists.
    pub(crate) public: Option<PathBuf>,
}

impl Deck {
    /// The deck root: the directory a slide's relative asset paths resolve
    /// against, and the Typst project root used when rendering.
    ///
    /// Slides reference assets as `../public/logo.png`, so this has to be the
    /// slides folder's parent. Compiling with any other root makes `typst`
    /// reject every such path with "would escape the project root".
    pub(crate) fn root(&self) -> PathBuf {
        self.slides
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map_or_else(|| self.slides.clone(), Path::to_path_buf)
    }
}

/// Resolves the slides folder and an optional sibling `public/` directory.
///
/// Accepts either the slides folder directly, or a deck root containing a
/// `slides/` subdirectory (with `public/` alongside it).
///
/// Every command routes through this so that `toboggan lint -p deck` and
/// `toboggan lint -p deck/slides` agree. They used to disagree silently: only the
/// serve path descended into `slides/`, so pointing `lint`, `stats`, `pdf` or
/// `thumbnails` at the deck root made them analyse a folder with no slides in it
/// — and `lint` then reported a clean deck and exited 0, which passes any CI
/// gate unconditionally. The scaffolding hands out the deck root (`toboggan new`
/// prints it, the generated `mise.toml` uses it), so that was the path users
/// were most likely to take.
pub(crate) fn resolve_deck(input: &Path) -> Deck {
    let nested_slides = input.join("slides");
    if nested_slides.is_dir() {
        let public = input.join("public");
        Deck {
            slides: nested_slides,
            public: public.is_dir().then_some(public),
        }
    } else {
        let public = input
            .parent()
            .map(|parent| parent.join("public"))
            .filter(|path| path.is_dir());
        Deck {
            slides: input.to_path_buf(),
            public,
        }
    }
}

/// Parses a deck and fails if any slide could not be processed.
///
/// [`toboggan_cli::ParseResult::to_talk`] drops unparseable slides, so without
/// this check a single front-matter typo silently removes a slide from the
/// linted, rendered, or served deck while the command still exits 0. Refusing is
/// the right default for one-shot commands; the watcher uses
/// [`build_talk_lossy`] instead so a typo mid-rehearsal does not tear down a
/// running server.
pub(crate) fn build_talk(slides: &Path, settings: &toboggan_cli::Settings) -> anyhow::Result<Talk> {
    let parse_result = toboggan_cli::parse_presentation(slides, settings)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    let errors = parse_result.errors();
    if !errors.is_empty() {
        anyhow::bail!(
            "{} slide(s) failed to parse:\n  {}",
            errors.len(),
            errors.join("\n  ")
        );
    }
    Ok(parse_result.to_talk())
}

/// Parses a deck, logging unparseable slides instead of failing.
///
/// Used by the live-reload watcher: dropping the whole server because the author
/// saved a half-written slide would be worse than serving the rest, but the
/// slide vanishing with no explanation was worse still.
pub(crate) fn build_talk_lossy(
    slides: &Path,
    settings: &toboggan_cli::Settings,
) -> anyhow::Result<Talk> {
    let parse_result = toboggan_cli::parse_presentation(slides, settings)
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    for error in parse_result.errors() {
        tracing::error!("slide dropped from the deck: {error}");
    }
    Ok(parse_result.to_talk())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// The regression this module exists for: pointing a command at the deck root
    /// used to make it analyse a folder with no slides and report success.
    #[test]
    fn deck_root_resolves_to_the_slides_folder() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        std::fs::create_dir(root.join("slides")).expect("slides");
        std::fs::create_dir(root.join("public")).expect("public");

        let deck = resolve_deck(root);
        assert_eq!(deck.slides, root.join("slides"));
        assert_eq!(deck.public, Some(root.join("public")));
        assert_eq!(deck.root(), root);
    }

    /// Pointing at the slides folder directly has to give the same answer, so
    /// `toboggan lint -p deck` and `toboggan lint -p deck/slides` agree.
    #[test]
    fn slides_folder_resolves_to_itself_and_finds_sibling_public() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let slides = root.join("slides");
        std::fs::create_dir(&slides).expect("slides");
        std::fs::create_dir(root.join("public")).expect("public");

        let deck = resolve_deck(&slides);
        assert_eq!(deck.slides, slides);
        assert_eq!(deck.public, Some(root.join("public")));
        assert_eq!(deck.root(), root);
    }

    #[test]
    fn a_bare_folder_has_no_public_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let deck = resolve_deck(temp.path());
        assert_eq!(deck.slides, temp.path());
        assert_eq!(deck.public, None);
    }

    /// A single relative segment has no usable parent; the deck is itself.
    #[test]
    fn root_of_a_bare_relative_path_is_the_path_itself() {
        let deck = Deck {
            slides: PathBuf::from("slides"),
            public: None,
        };
        assert_eq!(deck.root(), PathBuf::from("slides"));
    }
}
