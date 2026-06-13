use std::fs;
use std::path::{Path, PathBuf};

/// Safe mutation surface over a presentation folder.
///
/// All writes go through this type, which confines paths to the root, numbers
/// new parts/slides deterministically, and writes atomically (temp file +
/// rename within the same directory).
pub(crate) struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// Opens the workspace rooted at `root` (created if missing).
    ///
    /// # Errors
    /// Returns an error if the root cannot be created or canonicalized.
    pub(crate) fn new(root: &Path) -> anyhow::Result<Self> {
        fs::create_dir_all(root)?;
        let root = root.canonicalize()?;
        Ok(Self { root })
    }

    /// Creates a new section folder `NN-slug/` with a `_part.md`.
    ///
    /// # Errors
    /// Returns an error if the folder or file cannot be created.
    pub(crate) fn add_part(&self, title: &str) -> anyhow::Result<Vec<String>> {
        let number = Self::next_number(&self.root)?;
        let slug = slugify(title);
        let dir_name = format!("{number:02}-{slug}");
        let dir = self.confine(&self.root.join(&dir_name))?;
        fs::create_dir_all(&dir)?;

        let part_file = dir.join("_part.md");
        let content = format!(
            "+++\ntitle = \"{}\"\n+++\n\n# {}\n",
            escape_toml(title),
            title
        );
        atomic_write(&part_file, content.as_bytes())?;

        Ok(vec![format!("{dir_name}/_part.md")])
    }

    /// Creates a new slide `NN-slug.md` at the top level or inside `part`.
    ///
    /// # Errors
    /// Returns an error if `part` does not exist or the file cannot be written.
    pub(crate) fn add_slide(
        &self,
        part: Option<&str>,
        title: &str,
        body: Option<&str>,
    ) -> anyhow::Result<Vec<String>> {
        let target_dir = match part {
            Some(part) => {
                let dir = self.confine(&self.root.join(part))?;
                if !dir.is_dir() {
                    anyhow::bail!("section folder not found: {part}");
                }
                dir
            }
            None => self.root.clone(),
        };

        let number = Self::next_number(&target_dir)?;
        let slug = slugify(title);
        let file_name = format!("{number:02}-{slug}.md");
        let file = self.confine(&target_dir.join(&file_name))?;

        let body = body.unwrap_or("").trim_end();
        let content = format!("+++\ntitle = \"{}\"\n+++\n\n{}\n", escape_toml(title), body);
        atomic_write(&file, content.as_bytes())?;

        let rel = match part {
            Some(part) => format!("{part}/{file_name}"),
            None => file_name,
        };
        Ok(vec![rel])
    }

    /// Resolves `path` and asserts it stays within the workspace root.
    fn confine(&self, path: &Path) -> anyhow::Result<PathBuf> {
        // Resolve against the (existing) root; the target itself may not exist yet,
        // so canonicalize the parent and re-join the final component.
        let parent = path.parent().unwrap_or(&self.root);
        let name = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("invalid path: {}", path.display()))?;
        fs::create_dir_all(parent)?;
        let parent = parent.canonicalize()?;
        if !parent.starts_with(&self.root) {
            anyhow::bail!("path escapes the presentation folder: {}", path.display());
        }
        Ok(parent.join(name))
    }

    /// Returns the next `NN` number for a directory: max existing prefix + 1.
    fn next_number(dir: &Path) -> anyhow::Result<usize> {
        let mut max = 0;
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if let Some(number) = leading_number(&name) {
                    max = max.max(number);
                }
            }
        }
        Ok(max + 1)
    }
}

/// Parses a leading `NN` (digits before the first `-` or `_`) from a name.
fn leading_number(name: &str) -> Option<usize> {
    let digits = name
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    digits.parse::<usize>().ok()
}

/// Writes `bytes` to `path` atomically via a temp file in the same directory.
fn atomic_write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write as _;
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    temp.write_all(bytes)?;
    temp.persist(path)?;
    Ok(())
}

fn slugify(title: &str) -> String {
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
        "slide".to_owned()
    } else {
        trimmed
    }
}

fn escape_toml(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("My Great Slide!"), "my-great-slide");
        assert_eq!(slugify("***"), "slide");
    }

    #[test]
    fn leading_number_parses_prefix() {
        assert_eq!(leading_number("03-intro.md"), Some(3));
        assert_eq!(leading_number("12_section"), Some(12));
        assert_eq!(leading_number("_cover.md"), None);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn add_part_and_slide_number_sequentially() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = Workspace::new(dir.path()).expect("workspace");

        let part = workspace.add_part("Intro").expect("add part");
        assert_eq!(part, vec!["01-intro/_part.md".to_owned()]);

        let slide = workspace
            .add_slide(Some("01-intro"), "Hello", Some("# Hello\n\nbody"))
            .expect("add slide");
        assert_eq!(slide, vec!["01-intro/01-hello.md".to_owned()]);

        let top = workspace
            .add_slide(None, "Top Level", None)
            .expect("add top");
        // The part folder is 01-, so the next top-level number is 02.
        assert_eq!(top, vec!["02-top-level.md".to_owned()]);
    }
}
