use std::ffi::OsStr;
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, parse_document};
use miette::SourceSpan;
use toboggan_core::{Content, Date, Slide, SlideKind};
use tracing::debug;

use super::{
    DEFAULT_PART_TITLE, FRONT_MATTER_DELIMITER, FrontMatter, ParseContext, SlideContentParser,
    SlideContext, create_syntax_highlighter, default_options, default_plugins,
};
use crate::error::{Result, TobogganCliError};
use crate::{SlideProcessingResult, TalkMetadata, parse_date_string};

const FILE_MARKDOWN: &str = "md";
const FILE_MARKDOWN_FULL: &str = "markdown";
const FILE_HTML: &str = "html";
const FILE_HTM: &str = "htm";

const COVER: &str = "_cover.md";
const FOOTER: &str = "_footer.html";
const HEAD: &str = "_head.html";
const PREAMBLE: &str = "_preamble.typ";
const PART: &str = "_part.md";

#[derive(Debug, Clone)]
pub(super) struct TobogganDir {
    path: PathBuf,
}

impl TobogganDir {
    pub(super) fn new(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Err(TobogganCliError::NotADirectory { path });
        }
        if !path.is_dir() {
            return Err(TobogganCliError::NotADirectory { path });
        }

        debug!("Created TobogganDir for {}", path.display());
        Ok(Self { path })
    }

    pub(super) fn get_all_entries(&self) -> Result<Vec<DirEntry>> {
        read_sorted_directory(&self.path)
    }

    fn find_file(&self, filename: &str) -> Result<Option<DirEntry>> {
        let entries = self.get_all_entries()?;
        Ok(entries.into_iter().find(|entry| {
            entry.file_name().to_string_lossy() == filename && entry.path().is_file()
        }))
    }

    pub(super) fn get_cover(&self) -> Result<Option<DirEntry>> {
        self.find_file(COVER)
    }

    pub(super) fn get_footer(&self) -> Result<Option<DirEntry>> {
        self.find_file(FOOTER)
    }

    pub(super) fn get_head(&self) -> Result<Option<DirEntry>> {
        self.find_file(HEAD)
    }

    pub(super) fn get_typst_preamble(&self) -> Result<Option<DirEntry>> {
        self.find_file(PREAMBLE)
    }

    pub(super) fn get_part(&self) -> Result<Option<DirEntry>> {
        self.find_file(PART)
    }

    pub(super) fn get_slide_files(&self) -> Result<Vec<DirEntry>> {
        let entries = self.get_all_entries()?;
        let is_slide_file = |entry: &DirEntry| {
            let filename = entry.file_name();
            let filename = filename.to_string_lossy();
            let path = entry.path();

            !Self::should_skip_entry(&filename) && path.is_file() && is_slide_file(&path)
        };
        let result = entries.into_iter().filter(is_slide_file).collect();
        Ok(result)
    }

    pub(super) fn get_processable_entries(&self) -> Result<Vec<DirEntry>> {
        let entries = self.get_all_entries()?;
        let is_processable = |entry: &DirEntry| {
            let filename = entry.file_name();
            let filename = filename.to_string_lossy();
            let path = entry.path();

            if Self::should_skip_entry(&filename) {
                return false;
            }

            path.is_dir() || (path.is_file() && is_slide_file(&path))
        };
        let result = entries.into_iter().filter(is_processable).collect();
        Ok(result)
    }

    #[must_use]
    fn should_skip_entry(filename: &str) -> bool {
        filename.starts_with('.')
            || filename == COVER
            || filename == FOOTER
            || filename == HEAD
            || filename == PREAMBLE
            || filename == PART
    }
}

impl AsRef<Path> for TobogganDir {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

pub(super) fn read_sorted_directory(path: &Path) -> Result<Vec<DirEntry>> {
    let mut entries: Vec<_> = fs::read_dir(path)
        .map_err(|source| TobogganCliError::read_directory(path.to_path_buf(), source))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|source| TobogganCliError::read_directory(path.to_path_buf(), source))?;
    entries.sort_by_key(DirEntry::file_name);
    Ok(entries)
}

pub(super) fn is_slide_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|extension| extension.to_str()) {
        matches!(
            ext.to_lowercase().as_str(),
            FILE_MARKDOWN | FILE_MARKDOWN_FULL | FILE_HTML | FILE_HTM
        )
    } else {
        false
    }
}

pub(super) fn create_slide_from_file(
    file_path: &Path,
    ctx: ParseContext<'_>,
    asset_root: Option<&Path>,
) -> Result<(Slide, FrontMatter)> {
    let filename = file_path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("untitled");

    let slide_kind = match filename {
        "_cover" => SlideKind::Cover,
        "_part" => SlideKind::Part,
        _ => SlideKind::Standard,
    };

    let content = fs::read_to_string(file_path)
        .map_err(|source| TobogganCliError::read_file(file_path.to_path_buf(), source))?;

    let slide = if matches!(
        file_path.extension().and_then(|ext| ext.to_str()),
        Some(FILE_HTML | FILE_HTM)
    ) {
        let slide = create_html_slide(&content, slide_kind, filename, file_path);
        (slide, FrontMatter::default())
    } else {
        parse_slide_from_markdown(
            &content,
            slide_kind,
            Some(filename),
            Some(file_path),
            ctx,
            asset_root,
        )?
    };

    Ok(slide)
}

pub(super) fn parse_slide_from_markdown(
    content: &str,
    kind: SlideKind,
    filename: Option<&str>,
    file_path: Option<&Path>,
    ctx: ParseContext<'_>,
    asset_root: Option<&Path>,
) -> Result<(Slide, FrontMatter)> {
    let arena = Arena::new();
    let options = default_options();
    let highlighter = create_syntax_highlighter(ctx.theme);
    let mut plugins = default_plugins();
    plugins.render.codefence_syntax_highlighter = Some(&highlighter);

    let root = parse_document(&arena, content, &options);

    let content_parser = SlideContentParser::new();

    let (mut slide, front_matter) = content_parser.parse(
        root.children(),
        &options,
        &plugins,
        SlideContext {
            name: filename,
            path: file_path,
            asset_root,
            mermaid: ctx.mermaid,
        },
    )?;

    slide.kind = kind;

    Ok((slide, front_matter))
}

fn create_html_slide(
    content: &str,
    slide_kind: SlideKind,
    filename: &str,
    file_path: &Path,
) -> Slide {
    let html_content = Content::html(content.trim());

    let slide = match slide_kind {
        SlideKind::Cover => Slide::cover(filename),
        SlideKind::Part => Slide::part(filename),
        SlideKind::Standard => Slide::new(filename),
    };

    slide.with_body(html_content).with_source_path(file_path)
}

pub(super) fn parse_frontmatter(content: &str, file_path: &str) -> Result<FrontMatter> {
    let trimmed_content = content
        .trim()
        .trim_start_matches(FRONT_MATTER_DELIMITER)
        .trim_end_matches(FRONT_MATTER_DELIMITER);

    toml::from_str::<FrontMatter>(trimmed_content).map_err(|source| {
        // `toml` reports offsets into `trimmed_content`, but the snippet miette
        // draws is `content` — so they need rebasing. Every method used to
        // build `trimmed_content` returns a subslice, so its own address gives
        // the offset exactly, whatever combination of delimiters and
        // whitespace was stripped. Deriving it from `find("+++")` instead once
        // added the leading whitespace a second time, since
        // `trim_start_matches` strips the delimiter but keeps the newline after
        // it — which put the caret one column to the right of the offending key.
        let rebase = trimmed_content.as_ptr() as usize - content.as_ptr() as usize;
        let span = source.span().map_or_else(
            // No position from `toml`: highlight the whole block rather than
            // pointing somewhere arbitrary.
            || SourceSpan::from((0, content.len())),
            |span| SourceSpan::from((rebase + span.start, span.len())),
        );

        TobogganCliError::parse_frontmatter(
            file_path,
            content.to_owned(), // Use original content with delimiters for context
            span,
            source,
        )
    })
}

pub(super) fn extract_node_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();

    match &node.data.borrow().value {
        NodeValue::Text(content) => text.push_str(content),
        NodeValue::Code(code) => text.push_str(&code.literal),
        NodeValue::SoftBreak => text.push(' '),
        NodeValue::LineBreak => text.push('\n'),
        _ => {
            for child in node.children() {
                text.push_str(&extract_node_text(child));
            }
        }
    }

    text
}

/// The deck root a slides folder belongs to: the directory containing it.
///
/// Assets a slide references (`snippets/`, `public/`) sit beside `slides/`, not
/// inside it.
///
/// A relative folder with no directory component (`slides`) has an *empty*
/// parent rather than no parent, and the empty parent still means "the current
/// directory". Mapping it to `.` is what keeps `-p slides` and `-p ./slides/`
/// resolving assets against the same root; treating it as "no parent" used to
/// root the first one inside the slides folder itself.
fn deck_root(slides: &Path) -> PathBuf {
    match slides.parent() {
        // A filesystem root is its own parent.
        None => slides.to_path_buf(),
        Some(parent) if parent.as_os_str().is_empty() => PathBuf::from("."),
        Some(parent) => parent.to_path_buf(),
    }
}

pub(super) fn process_talk_metadata(
    toboggan_dir: &TobogganDir,
    ctx: ParseContext<'_>,
) -> Result<TalkMetadata> {
    let mut metadata = TalkMetadata {
        source_dir: Some(toboggan_dir.as_ref().to_path_buf()),
        ..TalkMetadata::default()
    };

    if let Some(cover) = toboggan_dir.get_cover()? {
        let path = cover.path();
        debug!("Processing cover slide: {}", path.display());
        let asset_root = deck_root(toboggan_dir.as_ref());
        let (cover_slide, front_matter) =
            create_slide_from_file(&path, ctx, Some(asset_root.as_path()))?;
        metadata.title = cover_slide.title.to_string();
        metadata.date = front_matter
            .date
            .and_then(|date| parse_date_string(&date).ok())
            .unwrap_or_else(Date::today);
        // The cover's frontmatter is the natural place for talk-level defaults.
        metadata.default_terminal_cwd = front_matter.quake_cwd;
        metadata.lang = front_matter.lang;
    }

    if let Some(footer) = toboggan_dir.get_footer()? {
        let path = footer.path();
        debug!("Processing footer: {}", path.display());
        let content = fs::read_to_string(&path)
            .map_err(|err| TobogganCliError::read_file(path.clone(), err))?;
        metadata.footer = Some(content);
    }

    if let Some(head) = toboggan_dir.get_head()? {
        let path = head.path();
        debug!("Processing head: {}", path.display());
        let content = fs::read_to_string(&path)
            .map_err(|err| TobogganCliError::read_file(path.clone(), err))?;
        metadata.head = Some(content);
    }

    if let Some(preamble) = toboggan_dir.get_typst_preamble()? {
        let path = preamble.path();
        debug!("Processing Typst preamble: {}", path.display());
        let content = fs::read_to_string(&path)
            .map_err(|err| TobogganCliError::read_file(path.clone(), err))?;
        // A blank file means "absent", not "replace the preamble with nothing":
        // `touch _preamble.typ` is a plausible first move, and taking it at its
        // word emits a `.typ` with no imports at all, which fails deep inside
        // typst complaining about a `slide` variable the author never wrote.
        metadata.typst_preamble = Some(content).filter(|content| !content.trim().is_empty());
    }

    Ok(metadata)
}

pub(super) fn process_all_entries(
    toboggan_dir: &TobogganDir,
    ctx: ParseContext<'_>,
) -> Result<Vec<SlideProcessingResult>> {
    // `<!-- code:lang:path -->` resolves against the deck root, i.e. the slides
    // folder's parent — that is where a deck's `snippets/` and `public/` live.
    let asset_root = deck_root(toboggan_dir.as_ref());
    let asset_root = Some(asset_root.as_path());
    let mut result = vec![];

    // Process cover slide first if it exists
    if let Some(cover) = toboggan_dir.get_cover()? {
        let path = cover.path();
        debug!("Processing cover slide: {}", path.display());
        let slide_result = process_single_file(&path, ctx, asset_root);
        result.push(slide_result);
    }

    let entries = toboggan_dir.get_processable_entries()?;

    for entry in entries {
        let path = entry.path();

        if path.is_dir() {
            let folder_results = process_folder_comprehensive(&path, ctx, asset_root)?;
            result.extend(folder_results);
        } else if is_slide_file(&path) {
            debug!("Processing file as slide: {}", path.display());
            let slide_result = process_single_file(&path, ctx, asset_root);
            result.push(slide_result);
        } else {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown file");
            result.push(SlideProcessingResult::Ignored(format!(
                "Non-slide file: {filename}"
            )));
        }
    }
    Ok(result)
}

fn process_single_file(
    path: &Path,
    ctx: ParseContext<'_>,
    asset_root: Option<&Path>,
) -> SlideProcessingResult {
    match create_slide_from_file(path, ctx, asset_root) {
        Ok((slide, front_matter)) => {
            if front_matter.skip {
                SlideProcessingResult::Skipped(slide)
            } else {
                SlideProcessingResult::Processed(slide)
            }
        }
        // Kept whole rather than flattened to a string: every variant that can
        // arrive here already names the file, and the ones carrying a snippet
        // can only draw it if the diagnostic survives this far.
        Err(error) => SlideProcessingResult::Error(Box::new(error)),
    }
}

fn process_folder_comprehensive(
    folder: &Path,
    ctx: ParseContext<'_>,
    asset_root: Option<&Path>,
) -> Result<Vec<SlideProcessingResult>> {
    let mut results = vec![];
    debug!("Processing folder as part: {}", folder.display());

    let toboggan_dir = TobogganDir::new(folder.to_path_buf())?;

    // Process part slide if it exists
    if let Some(part_entry) = toboggan_dir.get_part()? {
        let path = part_entry.path();
        let part_result = process_single_file(&path, ctx, asset_root);
        results.push(part_result);
    } else {
        // Create implicit part slide from folder name
        let folder_name = folder
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(DEFAULT_PART_TITLE);
        let part_slide = Slide::part(folder_name);
        results.push(SlideProcessingResult::Processed(part_slide));
    }

    // Process all content files in the folder
    for entry in toboggan_dir.get_slide_files()? {
        let path = entry.path();
        debug!("Processing folder content file: {}", path.display());
        results.push(process_single_file(&path, ctx, asset_root));
    }

    Ok(results)
}

#[cfg(test)]
pub(crate) fn create_test_file(dir: &Path, filename: &str, content: &str) -> std::io::Result<()> {
    fs::write(dir.join(filename), content)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::mermaid::MermaidRenderer;

    /// `-p slides` and `-p ./slides/` name the same folder, so they have to
    /// resolve `<!-- code:… -->` paths and `public/` against the same root.
    #[test]
    fn deck_root_is_the_containing_directory_however_the_folder_is_spelled() {
        assert_eq!(deck_root(Path::new("slides")), PathBuf::from("."));
        assert_eq!(deck_root(Path::new("./slides/")), PathBuf::from("."));
        assert_eq!(
            deck_root(Path::new("/tmp/deck/slides")),
            PathBuf::from("/tmp/deck")
        );
    }

    /// The caret has to land on the offending key itself.
    ///
    /// `toml` reports offsets into the delimiter-stripped block, so they need
    /// rebasing onto the text miette draws — and getting that arithmetic wrong
    /// is invisible without checking the bytes the span actually covers, which
    /// is why this asserts on the slice rather than on the rendered output.
    #[test]
    fn a_frontmatter_error_spans_the_offending_key() {
        // Each case keeps the same key on the same line, but varies what comes
        // before the block — which is what the rebase has to cope with.
        for (label, content) in [
            ("plain", "+++\ntitle = \"T\"\nbogus_key = true\n+++"),
            (
                "leading blank line",
                "\n+++\ntitle = \"T\"\nbogus_key = true\n+++",
            ),
            (
                "no trailing newline",
                "+++\ntitle = \"T\"\nbogus_key = true+++",
            ),
        ] {
            let error = parse_frontmatter(content, "s.md").expect_err(label);
            let TobogganCliError::ParseFrontmatter { src, span, .. } = &error else {
                panic!("{label}: unexpected variant: {error:?}");
            };
            let spanned = src
                .inner()
                .get(span.offset()..span.offset() + span.len())
                .unwrap_or_else(|| panic!("{label}: span {span:?} outside the source"));
            assert_eq!(spanned, "bogus_key", "{label}: caret landed on {spanned:?}");
        }
    }

    /// A bad *value* is reported where the value is, not where its key is.
    #[test]
    fn a_frontmatter_error_can_span_a_value() {
        let content = "+++\ntitle = \"T\"\nduration = [1, 2]\n+++";
        let error = parse_frontmatter(content, "s.md").expect_err("array is not a duration");
        let TobogganCliError::ParseFrontmatter { src, span, .. } = &error else {
            panic!("unexpected variant: {error:?}");
        };
        let spanned = src
            .inner()
            .get(span.offset()..span.offset() + span.len())
            .expect("span inside the source");
        assert!(
            content[..span.offset()].ends_with("duration = "),
            "caret landed on {spanned:?}, not the value"
        );
    }

    #[test]
    fn test_toboggan_dir_new() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let dir_path = temp_dir.path().to_path_buf();

        let toboggan_dir =
            TobogganDir::new(dir_path.clone()).expect("Failed to create TobogganDir");
        assert_eq!(toboggan_dir.as_ref(), dir_path.as_path());
    }

    #[test]
    fn test_toboggan_dir_new_invalid_path() {
        let invalid_path = PathBuf::from("/nonexistent/path");
        let result = TobogganDir::new(invalid_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_cover() -> Result<()> {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let dir_path = temp_dir.path();

        create_test_file(dir_path, "_cover.md", "# Cover").expect("Failed to create cover");
        create_test_file(dir_path, "slide.md", "# Slide").expect("Failed to create slide");

        let toboggan_dir = TobogganDir::new(dir_path.to_path_buf())?;
        let cover = toboggan_dir.get_cover()?;

        assert!(cover.is_some());
        assert_eq!(
            cover
                .expect("Cover should exist")
                .file_name()
                .to_string_lossy(),
            "_cover.md"
        );
        Ok(())
    }

    #[test]
    fn test_get_footer() -> Result<()> {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let dir_path = temp_dir.path();

        create_test_file(dir_path, "_footer.html", "<footer>Footer</footer>")
            .expect("Failed to create footer");

        let toboggan_dir = TobogganDir::new(dir_path.to_path_buf())?;
        let footer = toboggan_dir.get_footer()?;

        assert!(footer.is_some());
        assert_eq!(
            footer
                .expect("Footer should exist")
                .file_name()
                .to_string_lossy(),
            "_footer.html"
        );
        Ok(())
    }

    #[test]
    fn test_get_slide_files() -> Result<()> {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let dir_path = temp_dir.path();

        create_test_file(dir_path, "_cover.md", "# Cover").expect("Failed to create cover");
        create_test_file(dir_path, "slide1.md", "# Slide 1").expect("Failed to create slide1");
        create_test_file(dir_path, "slide2.md", "# Slide 2").expect("Failed to create slide2");
        create_test_file(dir_path, "not_a_slide.txt", "Text").expect("Failed to create text file");

        let toboggan_dir = TobogganDir::new(dir_path.to_path_buf())?;
        let slide_files = toboggan_dir.get_slide_files()?;

        assert_eq!(slide_files.len(), 2);
        let filenames: Vec<_> = slide_files
            .iter()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(filenames.contains(&"slide1.md".to_owned()));
        assert!(filenames.contains(&"slide2.md".to_owned()));
        assert!(!filenames.contains(&"_cover.md".to_owned()));
        Ok(())
    }

    #[test]
    fn test_should_skip_entry() {
        assert!(TobogganDir::should_skip_entry(".hidden"));
        assert!(TobogganDir::should_skip_entry("_cover.md"));
        assert!(TobogganDir::should_skip_entry("_footer.html"));
        assert!(TobogganDir::should_skip_entry("_head.html"));
        assert!(TobogganDir::should_skip_entry("_part.md"));

        assert!(!TobogganDir::should_skip_entry("slide1.md"));
        assert!(!TobogganDir::should_skip_entry("content.md"));
        assert!(!TobogganDir::should_skip_entry("title.md"));
        assert!(!TobogganDir::should_skip_entry("title.txt"));
    }

    #[test]
    fn test_is_slide_file() {
        use std::path::PathBuf;

        assert!(is_slide_file(&PathBuf::from("slide.md")));
        assert!(is_slide_file(&PathBuf::from("slide.markdown")));
        assert!(is_slide_file(&PathBuf::from("slide.html")));
        assert!(is_slide_file(&PathBuf::from("slide.htm")));

        assert!(!is_slide_file(&PathBuf::from("slide.txt")));
        assert!(!is_slide_file(&PathBuf::from("slide.pdf")));
        assert!(!is_slide_file(&PathBuf::from("slide")));
    }

    #[test]
    fn process_talk_metadata_captures_source_dir_and_quake_default() -> Result<()> {
        let temp_dir = tempdir().expect("temp");
        let dir_path = temp_dir.path();

        create_test_file(
            dir_path,
            "_cover.md",
            "+++\nquake_cwd = \"examples/api\"\n+++\n# Cover",
        )
        .expect("write cover");

        let toboggan_dir = TobogganDir::new(dir_path.to_path_buf())?;
        let metadata = process_talk_metadata(
            &toboggan_dir,
            ParseContext {
                theme: "base16-ocean.light",
                mermaid: &MermaidRenderer::default(),
            },
        )?;

        assert_eq!(metadata.source_dir.as_deref(), Some(dir_path));
        assert_eq!(
            metadata.default_terminal_cwd.as_deref(),
            Some("examples/api")
        );
        Ok(())
    }
}
