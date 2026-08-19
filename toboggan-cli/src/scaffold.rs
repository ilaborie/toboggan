//! Scaffolding for a new presentation folder.
//!
//! Shared by the `toboggan new` command and the `new_presentation` MCP tool so
//! both produce the same folder convention from one set of templates. Version
//! control initialization is intentionally *not* handled here — callers that want
//! it (the CLI's `new`) do it themselves.

use std::fs;
use std::path::Path;

use toboggan_core::Date;

use crate::error::{Result, TobogganCliError};

const COVER_TEMPLATE: &str = include_str!("../templates/_cover.md");
const PART_TEMPLATE: &str = include_str!("../templates/_part.md");
const SLIDE_TEMPLATE: &str = include_str!("../templates/01-welcome.md");
const HEAD_TEMPLATE: &str = include_str!("../templates/_head.html");
const GITIGNORE_TEMPLATE: &str = include_str!("../templates/gitignore");
const MISE_TEMPLATE: &str = include_str!("../templates/mise.toml");
const CONFIG_TEMPLATE: &str = include_str!("../templates/toboggan.toml");

/// Creates the standard presentation folder under `dir`:
/// `slides/` (with `_cover.md`, `_head.html`, and an `01-introduction/` part),
/// `public/`, `.gitignore`, `mise.toml`, and `toboggan.toml`.
///
/// # Errors
/// Returns an error if `dir` exists and is non-empty, or if any file or
/// directory cannot be created.
pub fn create_presentation(dir: &Path, title: &str, date: Date) -> Result<()> {
    if dir.is_dir()
        && fs::read_dir(dir)
            .map_err(|source| TobogganCliError::create_file(dir.to_path_buf(), source))?
            .next()
            .is_some()
    {
        return Err(TobogganCliError::scaffold(format!(
            "directory {} already exists and is not empty",
            dir.display()
        )));
    }

    let slides = dir.join("slides");
    let part = slides.join("01-introduction");
    create_dir(&part)?;
    create_dir(&dir.join("public"))?;

    let cover = COVER_TEMPLATE
        .replace("{{title}}", title)
        .replace("{{date}}", &date.to_string());
    write(&slides.join("_cover.md"), &cover)?;
    write(&slides.join("_head.html"), HEAD_TEMPLATE)?;
    write(&part.join("_part.md"), PART_TEMPLATE)?;
    write(&part.join("01-welcome.md"), SLIDE_TEMPLATE)?;
    write(&dir.join("public/.gitkeep"), "")?;
    write(&dir.join(".gitignore"), GITIGNORE_TEMPLATE)?;

    let mise = MISE_TEMPLATE.replace("{{slug}}", &slugify(title));
    write(&dir.join("mise.toml"), &mise)?;

    // The config doubles as the documentation for every setting: it lists each
    // key, commented out, with its default. Authors discover options by reading
    // the file they already have rather than by hunting through `--help`.
    let config = CONFIG_TEMPLATE
        .replace("{{title}}", title)
        .replace("{{date}}", &date.to_string());
    write(&dir.join("toboggan.toml"), &config)?;

    Ok(())
}

fn create_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .map_err(|source| TobogganCliError::create_file(path.to_path_buf(), source))
}

fn write(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content)
        .map_err(|source| TobogganCliError::write_file(path.to_path_buf(), source))
}

/// Lowercases and hyphen-joins alphanumeric runs, collapsing other characters.
#[must_use]
pub fn slugify(title: &str) -> String {
    let mut result = String::with_capacity(title.len());
    let mut last_dash = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            result.push('-');
            last_dash = true;
        }
    }
    let trimmed = result.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "presentation".to_owned()
    } else {
        trimmed
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

    #[test]
    #[allow(clippy::expect_used)]
    fn creates_the_folder_convention() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("deck");
        create_presentation(&root, "My Talk", Date::today()).expect("scaffold");

        assert!(root.join("slides/_cover.md").is_file());
        assert!(root.join("slides/_head.html").is_file());
        assert!(root.join("slides/01-introduction/_part.md").is_file());
        assert!(root.join("slides/01-introduction/01-welcome.md").is_file());
        assert!(root.join("public/.gitkeep").is_file());
        assert!(root.join(".gitignore").is_file());
        assert!(root.join("mise.toml").is_file());
    }

    /// The scaffold's templates carry `{{title}}`/`{{date}}`/`{{slug}}`
    /// placeholders. Asserting only that the files exist let a renamed
    /// placeholder ship a deck containing the literal `{{title}}`.
    #[test]
    #[allow(clippy::expect_used)]
    fn substitutes_every_template_placeholder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("deck");
        create_presentation(&root, "My Talk", Date::today()).expect("scaffold");

        for rel in [
            "slides/_cover.md",
            "slides/_head.html",
            "slides/01-introduction/_part.md",
            "slides/01-introduction/01-welcome.md",
            "mise.toml",
            ".gitignore",
        ] {
            let content = fs::read_to_string(root.join(rel)).expect("read");
            assert!(
                !content.contains("{{"),
                "{rel} still contains an unsubstituted placeholder:\n{content}"
            );
        }

        let cover = fs::read_to_string(root.join("slides/_cover.md")).expect("read");
        assert!(cover.contains("My Talk"), "cover lost the title:\n{cover}");
    }

    /// The single most valuable assertion here: a scaffolded deck must be a deck
    /// the parser accepts. Nothing else covered the two ends meeting.
    #[test]
    #[allow(clippy::expect_used)]
    fn the_scaffolded_deck_parses() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("deck");
        create_presentation(&root, "My Talk", Date::today()).expect("scaffold");

        let settings = crate::Settings {
            output: None,
            title: None,
            date: None,
            lang: None,
            base_url: None,
            theme: "base16-ocean.light".to_owned(),
            list_themes: false,
            format: None,
            no_counter: false,
            no_stats: true,
            wpm: 150,
            exclude_notes_from_duration: false,
            input: Some(root.join("slides")),
        };
        let parsed =
            crate::parse_presentation(&root.join("slides"), &settings).expect("parse scaffold");
        assert!(
            parsed.errors().is_empty(),
            "scaffolded deck does not parse: {:?}",
            parsed.errors()
        );
        let talk = parsed.to_talk();
        assert_eq!(talk.title, "My Talk");
        assert!(!talk.slides.is_empty(), "scaffolded deck has no slides");
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn rejects_a_non_empty_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("existing.txt"), "x").expect("write");
        let result = create_presentation(dir.path(), "Talk", Date::today());
        assert!(result.is_err());
    }
}
