use std::fs;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Serialize;
use toboggan_cli::parser::FRONT_MATTER_DELIMITER;
use toml_edit::{Array, DocumentMut, Item, Value};

/// Summary of the filesystem changes a mutation made (or would make on a dry run).
#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct ChangeSet {
    /// Files/folders created (relative to the workspace root).
    pub(crate) created: Vec<String>,
    /// Files modified in place.
    pub(crate) modified: Vec<String>,
    /// Files/folders deleted.
    pub(crate) deleted: Vec<String>,
    /// Renames, as `from -> to` (relative to the workspace root).
    pub(crate) renamed: Vec<String>,
    /// Whether this was a dry run (nothing was written).
    pub(crate) dry_run: bool,
}

impl ChangeSet {
    fn new(dry_run: bool) -> Self {
        Self {
            dry_run,
            ..Self::default()
        }
    }
}

/// A node in the folder-based outline: a cover, a part (with nested slides), or a
/// slide. `path` is relative to the workspace root and is what mutation tools
/// address.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct OutlineNode {
    /// Relative path (`NN-section` for parts, `NN-section/NN-slide.md` for slides).
    pub(crate) path: String,
    /// `"cover"`, `"part"`, or `"slide"`.
    pub(crate) kind: String,
    /// Front-matter title (falls back to the file/folder name).
    pub(crate) title: String,
    /// Render targets the slide is hidden in (e.g. `["web"]`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) hidden_in: Vec<String>,
    /// Whether the slide is skipped (excluded from the built talk).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) skip: bool,
    /// Nested slides (for parts).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) slides: Vec<OutlineNode>,
}

/// Safe mutation surface over a presentation folder.
///
/// All writes go through this type, which confines paths to the root, numbers
/// new parts/slides deterministically, writes atomically (temp file + rename
/// within the same directory), and edits front matter surgically with `toml_edit`
/// so comments and unknown keys survive.
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

    // ----- creation -------------------------------------------------------

    /// Creates a new section folder `NN-slug/` with a `_part.md`.
    pub(crate) fn add_part(&self, title: &str) -> anyhow::Result<ChangeSet> {
        let number = Self::next_number(&self.root)?;
        let dir_name = format!("{number:02}-{}", slugify(title));
        let dir = self.confine(&self.root.join(&dir_name))?;
        fs::create_dir_all(&dir)?;

        let part_file = dir.join("_part.md");
        let content = part_template(title);
        atomic_write(&part_file, content.as_bytes())?;

        let mut changes = ChangeSet::new(false);
        changes.created.push(format!("{dir_name}/_part.md"));
        Ok(changes)
    }

    /// Creates a new slide `NN-slug.md` at the top level or inside `part`.
    pub(crate) fn add_slide(
        &self,
        part: Option<&str>,
        title: &str,
        body: Option<&str>,
    ) -> anyhow::Result<ChangeSet> {
        let target_dir = self.section_dir(part)?;
        let number = Self::next_number(&target_dir)?;
        let file_name = format!("{number:02}-{}.md", slugify(title));
        let file = self.confine(&target_dir.join(&file_name))?;

        let content = slide_template(title, body.unwrap_or(""));
        atomic_write(&file, content.as_bytes())?;

        let mut changes = ChangeSet::new(false);
        changes.created.push(self.rel(&file));
        Ok(changes)
    }

    /// Scaffolds a complete new deck folder (its own `slides/`, `public/`,
    /// `mise.toml`) at the subpath `dir` relative to the root. `dir` must be a
    /// non-empty subdirectory name.
    pub(crate) fn new_presentation(
        &self,
        dir: &str,
        title: &str,
        date: toboggan_core::Date,
    ) -> anyhow::Result<ChangeSet> {
        if dir.is_empty() || dir == "." {
            anyhow::bail!("`dir` must be a subdirectory name for the new deck");
        }
        let target = self.confine(&self.root.join(dir))?;
        toboggan_cli::scaffold::create_presentation(&target, title, date)
            .map_err(|err| anyhow::anyhow!("{err}"))?;
        let mut changes = ChangeSet::new(false);
        changes.created.push(format!("{dir}/"));
        Ok(changes)
    }

    // ----- inspection -----------------------------------------------------

    /// Walks the folder and returns its outline (cover, parts, slides) with the
    /// relative paths the mutation tools address. This reflects the *files on
    /// disk* (including skipped slides), unlike the parsed `Talk`.
    pub(crate) fn outline(&self) -> anyhow::Result<Vec<OutlineNode>> {
        let mut nodes = Vec::new();
        let cover = self.root.join("_cover.md");
        if cover.is_file() {
            let fm = read_front_matter(&cover);
            nodes.push(OutlineNode {
                path: "_cover.md".to_owned(),
                kind: "cover".to_owned(),
                title: fm.title.unwrap_or_else(|| "Cover".to_owned()),
                hidden_in: Vec::new(),
                skip: fm.skip,
                slides: Vec::new(),
            });
        }

        for name in numbered_entries(&self.root)? {
            let path = self.root.join(&name);
            if path.is_dir() {
                let title = read_front_matter(&path.join("_part.md"))
                    .title
                    .unwrap_or_else(|| strip_leading_number(&name).to_owned());
                let slides = numbered_entries(&path)?
                    .into_iter()
                    .filter(|child| is_markdown(child))
                    .map(|child| slide_node(&format!("{name}/{child}"), &path.join(&child)))
                    .collect();
                nodes.push(OutlineNode {
                    path: name,
                    kind: "part".to_owned(),
                    title,
                    hidden_in: Vec::new(),
                    skip: false,
                    slides,
                });
            } else if is_markdown(&name) {
                nodes.push(slide_node(&name, &path));
            }
        }
        Ok(nodes)
    }

    // ----- front-matter / body edits -------------------------------------

    /// Sets the `title` front-matter key of a slide file.
    pub(crate) fn set_slide_title(
        &self,
        rel: &str,
        title: &str,
        dry_run: bool,
    ) -> anyhow::Result<ChangeSet> {
        self.edit_frontmatter(rel, dry_run, |doc| {
            assign(doc, "title", Value::from(title));
            Ok(())
        })
    }

    /// Sets the `title` of a section by editing its `_part.md`.
    pub(crate) fn set_part_title(
        &self,
        folder: &str,
        title: &str,
        dry_run: bool,
    ) -> anyhow::Result<ChangeSet> {
        let rel = format!("{}/_part.md", folder.trim_end_matches('/'));
        self.edit_frontmatter(&rel, dry_run, |doc| {
            assign(doc, "title", Value::from(title));
            Ok(())
        })
    }

    /// Replaces the markdown body of a slide, preserving its front matter.
    pub(crate) fn set_slide_body(
        &self,
        rel: &str,
        body: &str,
        dry_run: bool,
    ) -> anyhow::Result<ChangeSet> {
        let path = self.confine(&self.root.join(rel))?;
        let content = read(&path)?;
        let (fm, _) = split_front_matter(&content);
        let new_content = reassemble(fm.as_deref(), body);
        if !dry_run {
            atomic_write(&path, new_content.as_bytes())?;
        }
        Ok(self.modified(&path, dry_run))
    }

    /// Sets the `hidden_in` render targets of a slide (empty = visible
    /// everywhere). Targets must be `web` or `pdf`.
    pub(crate) fn set_hidden_in(
        &self,
        rel: &str,
        targets: &[String],
        dry_run: bool,
    ) -> anyhow::Result<ChangeSet> {
        for target in targets {
            if !matches!(target.as_str(), "web" | "pdf") {
                anyhow::bail!("invalid render target {target:?} (expected \"web\" or \"pdf\")");
            }
        }
        self.edit_frontmatter(rel, dry_run, |doc| {
            let value = (!targets.is_empty()).then(|| {
                let mut array = Array::new();
                for target in targets {
                    array.push(target.as_str());
                }
                Value::Array(array)
            });
            assign_or_remove(doc, "hidden_in", value);
            Ok(())
        })
    }

    /// Toggles the `skip` front-matter flag (a skipped slide is omitted entirely
    /// from the built talk).
    pub(crate) fn skip_slide(
        &self,
        rel: &str,
        skip: bool,
        dry_run: bool,
    ) -> anyhow::Result<ChangeSet> {
        self.edit_frontmatter(rel, dry_run, |doc| {
            assign_or_remove(doc, "skip", skip.then(|| Value::from(true)));
            Ok(())
        })
    }

    // ----- deletion -------------------------------------------------------

    /// Deletes a slide file.
    pub(crate) fn remove_slide(&self, rel: &str, dry_run: bool) -> anyhow::Result<ChangeSet> {
        let path = self.confine(&self.root.join(rel))?;
        if !path.is_file() {
            anyhow::bail!("slide not found: {rel}");
        }
        if !dry_run {
            fs::remove_file(&path)?;
        }
        let mut changes = ChangeSet::new(dry_run);
        changes.deleted.push(self.rel(&path));
        Ok(changes)
    }

    /// Deletes a section folder and all of its slides.
    pub(crate) fn remove_part(&self, folder: &str, dry_run: bool) -> anyhow::Result<ChangeSet> {
        let path = self.confine(&self.root.join(folder))?;
        if !path.is_dir() {
            anyhow::bail!("section folder not found: {folder}");
        }
        if !dry_run {
            fs::remove_dir_all(&path)?;
        }
        let mut changes = ChangeSet::new(dry_run);
        changes.deleted.push(format!("{}/", self.rel(&path)));
        Ok(changes)
    }

    // ----- reorder / move -------------------------------------------------

    /// Renumbers the entries of a directory into the given order. At the top
    /// level (`part = None`) this reorders parts and top-level slides; inside a
    /// part it reorders that part's slides. `order` must be a permutation of the
    /// directory's currently-numbered entries.
    pub(crate) fn reorder(
        &self,
        part: Option<&str>,
        order: &[String],
        dry_run: bool,
    ) -> anyhow::Result<ChangeSet> {
        let dir = self.section_dir(part)?;
        self.renumber(&dir, order, dry_run)
    }

    /// Moves a slide to another section (or the top level) at an optional 1-based
    /// position, renumbering both the source and target directories.
    pub(crate) fn move_slide(
        &self,
        from: &str,
        to_part: Option<&str>,
        position: Option<usize>,
        dry_run: bool,
    ) -> anyhow::Result<ChangeSet> {
        let source = self.confine(&self.root.join(from))?;
        if !source.is_file() {
            anyhow::bail!("slide not found: {from}");
        }
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("invalid slide path: {from}"))?
            .to_owned();
        let source_dir = source
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid slide path: {from}"))?
            .to_path_buf();
        let target_dir = self.section_dir(to_part)?;

        if source_dir == target_dir {
            anyhow::bail!("slide is already in the target section; use `reorder` instead");
        }

        // The moved file lands in the target dir with a guaranteed-free number.
        let slug = strip_leading_number(&file_name);
        let parked = format!("{:02}-{slug}", Self::next_number(&target_dir)?);

        // Derive the full plan from the *pre-move* directory snapshots so the
        // dry-run preview and the real apply share one source of truth (the move
        // itself plus the simulated post-move state of each directory).
        let target_before = numbered_entries(&target_dir)?;
        let insert_at = position.map_or(target_before.len(), |pos| {
            pos.saturating_sub(1).min(target_before.len())
        });
        let source_after = numbered_entries(&source_dir)?
            .into_iter()
            .filter(|name| name != &file_name)
            .collect::<Vec<_>>();
        let mut target_present = target_before.clone();
        target_present.push(parked.clone());
        let mut target_order = target_before;
        target_order.insert(insert_at, parked.clone());

        let mut changes = ChangeSet::new(dry_run);
        // Step 1: the move into the target directory.
        changes.renamed.push(self.rename_label(
            &target_dir,
            &file_name,
            &parked,
            Some(&source_dir),
        ));
        if !dry_run {
            fs::rename(&source, target_dir.join(&parked))?;
        }

        // Steps 2 & 3: renumber both directories. Renumber the NESTED directory
        // before its ancestor — renumbering a parent renames the child's folder,
        // invalidating its path. The same ordering drives the dry-run preview, so
        // the previewed renames match what the apply produces.
        let renumber_source =
            |this: &Self| this.renumber_entries(&source_dir, &source_after, &source_after, dry_run);
        let renumber_target = |this: &Self| {
            this.renumber_entries(&target_dir, &target_present, &target_order, dry_run)
        };
        let (first, second) = if source_dir.starts_with(&target_dir) {
            (renumber_source(self)?, renumber_target(self)?)
        } else {
            (renumber_target(self)?, renumber_source(self)?)
        };
        changes.renamed.extend(first.renamed);
        changes.renamed.extend(second.renamed);
        Ok(changes)
    }

    // ----- internals ------------------------------------------------------

    /// Resolves the directory for an optional section name (`None` = root).
    fn section_dir(&self, part: Option<&str>) -> anyhow::Result<PathBuf> {
        match part {
            Some(part) => {
                let dir = self.confine(&self.root.join(part))?;
                if !dir.is_dir() {
                    anyhow::bail!("section folder not found: {part}");
                }
                Ok(dir)
            }
            None => Ok(self.root.clone()),
        }
    }

    /// Reads, edits, and re-writes a slide file's front matter via `toml_edit`,
    /// preserving the body, comments, and unknown keys.
    fn edit_frontmatter<F>(&self, rel: &str, dry_run: bool, edit: F) -> anyhow::Result<ChangeSet>
    where
        F: FnOnce(&mut DocumentMut) -> anyhow::Result<()>,
    {
        let path = self.confine(&self.root.join(rel))?;
        let content = read(&path)?;
        let (fm, body) = split_front_matter(&content);
        let mut doc = fm
            .as_deref()
            .unwrap_or("")
            .parse::<DocumentMut>()
            .map_err(|err| anyhow::anyhow!("invalid front matter in {rel}: {err}"))?;
        edit(&mut doc)?;
        let new_fm = doc.to_string();
        let new_content = reassemble(Some(&new_fm), &body);
        if !dry_run {
            atomic_write(&path, new_content.as_bytes())?;
        }
        Ok(self.modified(&path, dry_run))
    }

    /// Renumbers a directory's currently-numbered entries into `order`. Thin
    /// wrapper over [`Self::renumber_entries`] that reads the present entries.
    fn renumber(&self, dir: &Path, order: &[String], dry_run: bool) -> anyhow::Result<ChangeSet> {
        let present = numbered_entries(dir)?;
        self.renumber_entries(dir, &present, order, dry_run)
    }

    /// Two-phase renumber of `present` into `order` (1-based `NN-` prefixes),
    /// preserving each entry's slug. `present` is supplied explicitly so callers
    /// can describe a simulated post-move directory (see [`Self::move_slide`])
    /// without the on-disk listing having to match yet on a dry run. Routes every
    /// entry through a temporary name first so swaps never collide.
    fn renumber_entries(
        &self,
        dir: &Path,
        present: &[String],
        order: &[String],
        dry_run: bool,
    ) -> anyhow::Result<ChangeSet> {
        let renames = plan_renumber(present, order)?;

        let mut changes = ChangeSet::new(dry_run);
        self.record_renames(&mut changes, dir, &renames);
        if dry_run {
            return Ok(changes);
        }

        // Phase 1: park every entry under a unique temp name.
        for (index, (from, _)) in renames.iter().enumerate() {
            fs::rename(dir.join(from), dir.join(format!(".__renumber_{index}")))?;
        }
        // Phase 2: place each parked entry at its final name.
        for (index, (_, to)) in renames.iter().enumerate() {
            fs::rename(dir.join(format!(".__renumber_{index}")), dir.join(to))?;
        }
        Ok(changes)
    }

    /// Records the non-identity renames in `renames` as root-relative labels.
    fn record_renames(&self, changes: &mut ChangeSet, dir: &Path, renames: &[(String, String)]) {
        for (from, to) in renames {
            if from != to {
                changes.renamed.push(self.rename_label(dir, from, to, None));
            }
        }
    }

    /// Formats a `from -> to` rename relative to the workspace root. `from_dir`
    /// overrides the source directory (used by `move_slide`).
    fn rename_label(&self, dir: &Path, from: &str, to: &str, from_dir: Option<&Path>) -> String {
        let from_path = from_dir.unwrap_or(dir).join(from);
        format!("{} -> {}", self.rel(&from_path), self.rel(&dir.join(to)))
    }

    /// Resolves `path` (always `self.root.join(rel)`) and asserts it stays within
    /// the workspace root, so a hostile input can never create or write files
    /// outside the root.
    ///
    /// Three escape vectors are rejected: an absolute `rel` (whose `join`
    /// discards the root, leaving a path outside it), a `..` component, and a
    /// symlinked ancestor or final component that points out of the root.
    ///
    /// This is pure validation — it touches the filesystem only to read. Callers
    /// that write into a directory that may not exist create it themselves
    /// (`add_part`, `new_presentation`); every other caller targets the root or a
    /// section folder already known to exist. Resolving a path must not have side
    /// effects, or a failed lookup (`remove_slide` on a typo) would litter the
    /// deck with empty directories.
    fn confine(&self, path: &Path) -> anyhow::Result<PathBuf> {
        if !path.starts_with(&self.root) {
            anyhow::bail!("path escapes the presentation folder: {}", path.display());
        }
        if path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            anyhow::bail!("path must not contain `..`: {}", path.display());
        }
        path.file_name()
            .ok_or_else(|| anyhow::anyhow!("invalid path: {}", path.display()))?;
        let parent = path.parent().unwrap_or(&self.root);

        // Resolve the nearest existing ancestor and confirm it stays in the root
        // — guards against a symlinked ancestor.
        let anchor = nearest_existing(parent).canonicalize()?;
        if !anchor.starts_with(&self.root) {
            anyhow::bail!("path escapes the presentation folder: {}", path.display());
        }
        // Refuse to write through a pre-existing symlink as the final component.
        if path
            .symlink_metadata()
            .is_ok_and(|meta| meta.file_type().is_symlink())
        {
            anyhow::bail!("refusing to write through a symlink: {}", path.display());
        }

        Ok(path.to_path_buf())
    }

    /// Path relative to the workspace root, as a forward-slash string.
    fn rel(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    fn modified(&self, path: &Path, dry_run: bool) -> ChangeSet {
        let mut changes = ChangeSet::new(dry_run);
        changes.modified.push(self.rel(path));
        changes
    }

    /// Returns the next `NN` number for a directory: max existing prefix + 1.
    fn next_number(dir: &Path) -> anyhow::Result<usize> {
        if !dir.is_dir() {
            return Ok(1);
        }
        let max = numbered_entries(dir)?
            .iter()
            .filter_map(|name| leading_number(name))
            .max()
            .unwrap_or(0);
        Ok(max + 1)
    }
}

/// Lists a directory's entries that carry a leading `NN-` number, sorted by that
/// number. Non-numbered helpers (`_cover.md`, `_part.md`, `_head.html`, …) and
/// hidden temp files are excluded.
fn numbered_entries(dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(number) = leading_number(&name) {
            entries.push((number, name));
        }
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    Ok(entries.into_iter().map(|(_, name)| name).collect())
}

/// Parses a leading `NN` (digits before the first `-` or `_`) from a name.
fn leading_number(name: &str) -> Option<usize> {
    let digits = name
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<usize>().ok()
}

/// Strips a leading `NN-`/`NN_` number, returning the slug + extension.
fn strip_leading_number(name: &str) -> &str {
    let rest = name.trim_start_matches(|ch: char| ch.is_ascii_digit());
    rest.strip_prefix(['-', '_']).unwrap_or(rest)
}

/// Walks up from `path` to the first ancestor that exists on disk (so it can be
/// canonicalized to resolve any symlinks before we create the rest).
fn nearest_existing(path: &Path) -> PathBuf {
    let mut current = path;
    loop {
        if current.exists() {
            return current.to_path_buf();
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return current.to_path_buf(),
        }
    }
}

/// Assigns `new` to `key`, preserving the existing value's surrounding
/// formatting and comments (decor) when the key already holds a value.
fn assign(doc: &mut DocumentMut, key: &str, mut new: Value) {
    if let Some(Item::Value(existing)) = doc.get_mut(key) {
        *new.decor_mut() = existing.decor().clone();
        *existing = new;
    } else {
        doc.insert(key, Item::Value(new));
    }
}

/// Assigns `value` to `key` (preserving decor), or removes `key` when `None`.
fn assign_or_remove(doc: &mut DocumentMut, key: &str, value: Option<Value>) {
    match value {
        Some(value) => assign(doc, key, value),
        None => {
            doc.as_table_mut().remove(key);
        }
    }
}

/// Computes the `(current_name, target_name)` renames that place `order` into
/// 1-based `NN-` numbering, preserving slugs. `order` must be a permutation of
/// `present`.
fn plan_renumber(present: &[String], order: &[String]) -> anyhow::Result<Vec<(String, String)>> {
    if order.len() != present.len() || !is_permutation(order, present) {
        anyhow::bail!(
            "order must be a permutation of the directory's numbered entries {present:?}, got {order:?}"
        );
    }
    Ok(order
        .iter()
        .enumerate()
        .map(|(index, name)| {
            (
                name.clone(),
                format!("{:02}-{}", index + 1, strip_leading_number(name)),
            )
        })
        .collect())
}

fn is_permutation(left: &[String], right: &[String]) -> bool {
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    left.sort();
    right.sort();
    left == right
}

/// Splits a slide file into its TOML front matter (without the `+++` fences) and
/// the markdown body. Returns `(None, content)` when there is no front matter.
fn split_front_matter(content: &str) -> (Option<String>, String) {
    let mut lines = content.lines();
    match lines.next() {
        Some(line) if line.trim_end() == FRONT_MATTER_DELIMITER => {}
        _ => return (None, content.to_owned()),
    }

    let mut front = Vec::new();
    let mut closed = false;
    for line in lines.by_ref() {
        if line.trim_end() == FRONT_MATTER_DELIMITER {
            closed = true;
            break;
        }
        front.push(line);
    }
    if !closed {
        return (None, content.to_owned());
    }

    let body = lines.collect::<Vec<_>>().join("\n");
    (Some(front.join("\n")), body)
}

/// Reassembles a slide file from optional front matter and a body, following the
/// `+++\n<fm>\n+++\n\n<body>\n` convention.
fn reassemble(front: Option<&str>, body: &str) -> String {
    let body = body.trim_matches('\n');
    let fm = front.map(str::trim_end).filter(|fm| !fm.is_empty());
    match (fm, body.is_empty()) {
        (Some(fm), true) => format!("{FRONT_MATTER_DELIMITER}\n{fm}\n{FRONT_MATTER_DELIMITER}\n"),
        (Some(fm), false) => {
            format!("{FRONT_MATTER_DELIMITER}\n{fm}\n{FRONT_MATTER_DELIMITER}\n\n{body}\n")
        }
        (None, true) => String::new(),
        (None, false) => format!("{body}\n"),
    }
}

/// Builds an outline node for a slide file at relative path `rel`.
fn slide_node(rel: &str, path: &Path) -> OutlineNode {
    let fm = read_front_matter(path);
    OutlineNode {
        path: rel.to_owned(),
        kind: "slide".to_owned(),
        title: fm
            .title
            .unwrap_or_else(|| strip_leading_number(rel).trim_end_matches(".md").to_owned()),
        hidden_in: fm
            .hidden_in
            .iter()
            .filter_map(|target| render_target_str(*target))
            .collect(),
        skip: fm.skip,
        slides: Vec::new(),
    }
}

/// The serialized name of a render target (`"web"` / `"pdf"`). Derived from the
/// type's own `Serialize` impl so it stays correct as `RenderTarget`
/// (a `#[non_exhaustive]` enum) gains variants or changes its serde casing.
fn render_target_str(target: toboggan_core::RenderTarget) -> Option<String> {
    serde_json::to_value(target)
        .ok()?
        .as_str()
        .map(str::to_owned)
}

/// Whether `name` is a markdown file (case-insensitive extension).
fn is_markdown(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// Reads and parses a slide's front matter, returning the default on any error
/// (missing file, no front matter, invalid TOML) — outline is best-effort.
fn read_front_matter(path: &Path) -> toboggan_cli::parser::FrontMatter {
    let Ok(content) = fs::read_to_string(path) else {
        return toboggan_cli::parser::FrontMatter::default();
    };
    let (fm, _) = split_front_matter(&content);
    fm.and_then(|fm| toml::from_str(&fm).ok())
        .unwrap_or_default()
}

fn read(path: &Path) -> anyhow::Result<String> {
    fs::read_to_string(path).map_err(|err| anyhow::anyhow!("cannot read {}: {err}", path.display()))
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

fn part_template(title: &str) -> String {
    let fm = title_front_matter(title);
    format!("{FRONT_MATTER_DELIMITER}\n{fm}{FRONT_MATTER_DELIMITER}\n\n# {title}\n")
}

fn slide_template(title: &str, body: &str) -> String {
    let body = body.trim_end();
    let fm = title_front_matter(title);
    format!("{FRONT_MATTER_DELIMITER}\n{fm}{FRONT_MATTER_DELIMITER}\n\n{body}\n")
}

/// Renders a `title = "..."` front-matter block (with trailing newline) via
/// `toml_edit`, so titles with quotes, backslashes, or control characters are
/// escaped into valid TOML instead of corrupting the file.
fn title_front_matter(title: &str) -> String {
    let mut doc = DocumentMut::new();
    doc.insert("title", Item::Value(Value::from(title)));
    doc.to_string()
}

fn slugify(title: &str) -> String {
    toboggan_cli::scaffold::slugify(title)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn workspace() -> (tempfile::TempDir, Workspace) {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = Workspace::new(dir.path()).expect("workspace");
        (dir, workspace)
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("My Great Slide!"), "my-great-slide");
        assert_eq!(slugify("***"), "presentation");
    }

    #[test]
    fn leading_number_parses_prefix() {
        assert_eq!(leading_number("03-intro.md"), Some(3));
        assert_eq!(leading_number("12_section"), Some(12));
        assert_eq!(leading_number("_cover.md"), None);
    }

    #[test]
    fn strip_leading_number_keeps_slug_and_ext() {
        assert_eq!(strip_leading_number("03-intro.md"), "intro.md");
        assert_eq!(strip_leading_number("12_section"), "section");
        assert_eq!(strip_leading_number("plain.md"), "plain.md");
    }

    #[test]
    fn split_and_reassemble_round_trips() {
        let content = "+++\ntitle = \"Hi\" # keep me\nextra = 1\n+++\n\n# Body\n\ntext\n";
        let (fm, body) = split_front_matter(content);
        let fm = fm.expect("front matter");
        assert!(fm.contains("# keep me"));
        assert!(fm.contains("extra = 1"));
        assert_eq!(body.trim(), "# Body\n\ntext");
        let rebuilt = reassemble(Some(&fm), &body);
        assert!(rebuilt.starts_with("+++\n"));
        assert!(rebuilt.contains("extra = 1"));
    }

    #[test]
    fn no_front_matter_is_recognized() {
        let (fm, body) = split_front_matter("# Just a body\n");
        assert!(fm.is_none());
        assert_eq!(body, "# Just a body\n");
    }

    #[test]
    fn add_part_and_slide_number_sequentially() {
        let (_dir, workspace) = workspace();
        let part = workspace.add_part("Intro").expect("add part");
        assert_eq!(part.created, vec!["01-intro/_part.md".to_owned()]);

        let slide = workspace
            .add_slide(Some("01-intro"), "Hello", Some("# Hello\n\nbody"))
            .expect("add slide");
        assert_eq!(slide.created, vec!["01-intro/01-hello.md".to_owned()]);

        let top = workspace.add_slide(None, "Top", None).expect("add top");
        assert_eq!(top.created, vec!["02-top.md".to_owned()]);
    }

    #[test]
    fn set_title_preserves_comment_and_unknown_key() {
        let (dir, workspace) = workspace();
        let file = dir.path().join("01-x.md");
        fs::write(
            &file,
            "+++\ntitle = \"Old\" # a note\ncustom = true\n+++\n\nbody\n",
        )
        .expect("write");

        workspace
            .set_slide_title("01-x.md", "New", false)
            .expect("set title");

        let updated = fs::read_to_string(&file).expect("read");
        assert!(updated.contains("\"New\""));
        assert!(
            updated.contains("# a note"),
            "comment must survive: {updated}"
        );
        assert!(
            updated.contains("custom = true"),
            "unknown key must survive"
        );
        assert!(updated.contains("body"));
    }

    #[test]
    fn set_hidden_in_and_clear() {
        let (dir, workspace) = workspace();
        let file = dir.path().join("01-x.md");
        fs::write(&file, "+++\ntitle = \"X\"\n+++\n\nbody\n").expect("write");

        workspace
            .set_hidden_in("01-x.md", &["web".to_owned()], false)
            .expect("hide");
        assert!(
            fs::read_to_string(&file)
                .expect("read")
                .contains("hidden_in")
        );

        workspace
            .set_hidden_in("01-x.md", &[], false)
            .expect("show");
        assert!(
            !fs::read_to_string(&file)
                .expect("read")
                .contains("hidden_in")
        );
    }

    #[test]
    fn set_hidden_in_rejects_unknown_target() {
        let (dir, workspace) = workspace();
        fs::write(dir.path().join("01-x.md"), "+++\ntitle = \"X\"\n+++\n\nb\n").expect("write");
        let result = workspace.set_hidden_in("01-x.md", &["mobile".to_owned()], false);
        assert!(result.is_err());
    }

    #[test]
    fn reorder_reverses_numbering() {
        let (dir, workspace) = workspace();
        for name in ["01-a.md", "02-b.md", "03-c.md"] {
            fs::write(dir.path().join(name), "+++\ntitle=\"x\"\n+++\n\nb\n").expect("write");
        }
        workspace
            .reorder(
                None,
                &[
                    "03-c.md".to_owned(),
                    "02-b.md".to_owned(),
                    "01-a.md".to_owned(),
                ],
                false,
            )
            .expect("reorder");
        let names = numbered_entries(dir.path()).expect("list");
        assert_eq!(names, vec!["01-c.md", "02-b.md", "03-a.md"]);
    }

    #[test]
    fn reorder_dry_run_writes_nothing() {
        let (dir, workspace) = workspace();
        for name in ["01-a.md", "02-b.md"] {
            fs::write(dir.path().join(name), "x").expect("write");
        }
        let changes = workspace
            .reorder(None, &["02-b.md".to_owned(), "01-a.md".to_owned()], true)
            .expect("dry run");
        assert!(changes.dry_run);
        assert!(!changes.renamed.is_empty());
        // Files unchanged.
        assert_eq!(
            numbered_entries(dir.path()).expect("list"),
            vec!["01-a.md", "02-b.md"]
        );
    }

    #[test]
    fn reorder_rejects_bad_order() {
        let (dir, workspace) = workspace();
        fs::write(dir.path().join("01-a.md"), "x").expect("write");
        let result = workspace.reorder(None, &["09-missing.md".to_owned()], false);
        assert!(result.is_err());
    }

    #[test]
    fn move_slide_between_parts_renumbers_both() {
        let (dir, workspace) = workspace();
        let intro = dir.path().join("01-intro");
        let outro = dir.path().join("02-outro");
        fs::create_dir_all(&intro).expect("mkdir");
        fs::create_dir_all(&outro).expect("mkdir");
        for name in ["01-a.md", "02-b.md"] {
            fs::write(intro.join(name), "+++\ntitle=\"x\"\n+++\n\nb\n").expect("write");
        }
        fs::write(outro.join("01-z.md"), "+++\ntitle=\"z\"\n+++\n\nb\n").expect("write");

        workspace
            .move_slide("01-intro/02-b.md", Some("02-outro"), Some(1), false)
            .expect("move");

        let intro_names = numbered_entries(&intro).expect("list intro");
        assert_eq!(intro_names, vec!["01-a.md"]);
        let outro_names = numbered_entries(&outro).expect("list outro");
        assert_eq!(outro_names, vec!["01-b.md", "02-z.md"]);
    }

    #[test]
    fn move_slide_to_top_level_renames_parent_part() {
        let (dir, workspace) = workspace();
        let part = dir.path().join("01-intro");
        fs::create_dir_all(&part).expect("mkdir");
        for name in ["01-a.md", "02-b.md"] {
            fs::write(part.join(name), "+++\ntitle=\"x\"\n+++\n\nb\n").expect("write");
        }

        // Moving to the top level renumbers the root, renaming `01-intro` itself.
        workspace
            .move_slide("01-intro/02-b.md", None, Some(1), false)
            .expect("move to top");

        let top = numbered_entries(dir.path()).expect("list root");
        assert_eq!(top, vec!["01-b.md", "02-intro"]);
        let intro = numbered_entries(&dir.path().join("02-intro")).expect("list intro");
        assert_eq!(intro, vec!["01-a.md"]);
    }

    #[test]
    fn move_slide_from_top_level_into_part() {
        let (dir, workspace) = workspace();
        // Top-level slides 01-a, 02-b and a part 03-sec with one slide.
        for name in ["01-a.md", "02-b.md"] {
            fs::write(dir.path().join(name), "+++\ntitle=\"x\"\n+++\n\nb\n").expect("write");
        }
        let sec = dir.path().join("03-sec");
        fs::create_dir_all(&sec).expect("mkdir");
        fs::write(sec.join("01-z.md"), "+++\ntitle=\"z\"\n+++\n\nb\n").expect("write");

        // Moving a top-level slide into the part renumbers root (renaming 03-sec).
        workspace
            .move_slide("01-a.md", Some("03-sec"), Some(1), false)
            .expect("move into part");

        // Root compacted: 02-b -> 01-b, 03-sec -> 02-sec.
        let top = numbered_entries(dir.path()).expect("list root");
        assert_eq!(top, vec!["01-b.md", "02-sec"]);
        let sec_names = numbered_entries(&dir.path().join("02-sec")).expect("list sec");
        assert_eq!(sec_names, vec!["01-a.md", "02-z.md"]);
    }

    #[test]
    fn confine_rejects_parent_traversal_without_side_effects() {
        let (dir, workspace) = workspace();
        let result = workspace.set_slide_title("../evil.md", "x", false);
        assert!(result.is_err());
        // Nothing was created outside the root.
        assert!(!dir.path().join("../evil.md").exists());
        assert!(
            !dir.path()
                .parent()
                .expect("parent")
                .join("evil.md")
                .exists()
        );
    }

    #[test]
    fn confine_does_not_create_directories_while_resolving() {
        let (dir, workspace) = workspace();
        // A well-formed but non-existent path: rejected on its own merits, and
        // resolving it must leave the deck folder untouched.
        let result = workspace.remove_slide("ghost/part/slide.md", false);
        assert!(result.is_err());
        assert!(
            !dir.path().join("ghost").exists(),
            "resolving a path must not create directories"
        );
    }

    #[test]
    fn confine_rejects_absolute_path_without_side_effects() {
        let (_dir, workspace) = workspace();
        let escape = std::env::temp_dir().join("toboggan-confine-escape");
        let target = escape.join("evil.md");
        // An absolute `rel` would replace the root in `join`; it must be rejected
        // before any directory is created.
        let result = workspace.set_slide_title(&target.to_string_lossy(), "x", false);
        assert!(result.is_err());
        assert!(!escape.exists(), "no directory created outside the root");
    }

    #[test]
    fn move_slide_dry_run_matches_apply() {
        // The to-top-level case (source nested under target) is where the preview
        // and apply orderings used to diverge.
        let build = |dir: &Path| {
            let part = dir.join("01-intro");
            fs::create_dir_all(&part).expect("mkdir");
            for name in ["01-a.md", "02-b.md"] {
                fs::write(part.join(name), "+++\ntitle=\"x\"\n+++\n\nb\n").expect("write");
            }
        };

        let preview_dir = tempfile::tempdir().expect("tempdir");
        let preview_ws = Workspace::new(preview_dir.path()).expect("workspace");
        build(preview_dir.path());
        let preview = preview_ws
            .move_slide("01-intro/02-b.md", None, Some(1), true)
            .expect("dry run");

        let apply_dir = tempfile::tempdir().expect("tempdir");
        let apply_ws = Workspace::new(apply_dir.path()).expect("workspace");
        build(apply_dir.path());
        let applied = apply_ws
            .move_slide("01-intro/02-b.md", None, Some(1), false)
            .expect("apply");

        assert!(preview.dry_run && !applied.dry_run);
        assert_eq!(
            preview.renamed, applied.renamed,
            "dry-run preview must match the applied renames"
        );
    }

    #[test]
    fn add_slide_title_with_control_chars_stays_valid_toml() {
        let (dir, workspace) = workspace();
        // A title with a quote, backslash, and newline would break a hand-built
        // TOML string; toml_edit must escape it so the file re-parses.
        workspace
            .add_slide(None, "Tricky \"q\" \\ \n line", Some("body"))
            .expect("add slide");
        let created = numbered_entries(dir.path()).expect("list");
        let slide = created.first().expect("a slide");
        let content = fs::read_to_string(dir.path().join(slide)).expect("read");
        let (fm, _) = split_front_matter(&content);
        let fm = fm.expect("front matter");
        let parsed = fm
            .parse::<DocumentMut>()
            .expect("front matter must be valid TOML");
        assert_eq!(
            parsed.get("title").and_then(Item::as_str),
            Some("Tricky \"q\" \\ \n line"),
            "title round-trips through escaping"
        );
    }
}
