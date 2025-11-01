use std::ffi::OsStr;
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, parse_document};
use miette::SourceSpan;
use toboggan_core::{Content, Date, Slide, SlideKind};
use tracing::debug;

use super::{
    DEFAULT_PART_TITLE, FRONT_MATTER_DELIMITER, FrontMatter, SlideContentParser,
    create_syntax_highlighter, default_options, default_plugins,
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
    theme: &str,
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
        let slide = create_html_slide(&content, slide_kind, filename);
        (slide, FrontMatter::default())
    } else {
        parse_slide_from_markdown(&content, slide_kind, Some(filename), Some(file_path), theme)?
    };

    Ok(slide)
}

pub(super) fn parse_slide_from_markdown(
    content: &str,
    kind: SlideKind,
    filename: Option<&str>,
    file_path: Option<&Path>,
    theme: &str,
) -> Result<(Slide, FrontMatter)> {
    let arena = Arena::new();
    let options = default_options();
    let highlighter = create_syntax_highlighter(theme);
    let mut plugins = default_plugins();
    plugins.render.codefence_syntax_highlighter = Some(&highlighter);

    let root = parse_document(&arena, content, &options);

    let content_parser = SlideContentParser::new();

    let (mut slide, front_matter) =
        content_parser.parse(root.children(), &options, &plugins, filename, file_path)?;

    slide.kind = kind;

    Ok((slide, front_matter))
}

fn create_html_slide(content: &str, slide_kind: SlideKind, filename: &str) -> Slide {
    let html_content = Content::html(content.trim());

    let slide = match slide_kind {
        SlideKind::Cover => Slide::cover(filename),
        SlideKind::Part => Slide::part(filename),
        SlideKind::Standard => Slide::new(filename),
    };

    slide.with_body(html_content)
}

pub(super) fn parse_frontmatter(content: &str, file_path: &str) -> Result<FrontMatter> {
    let trimmed_content = content
        .trim()
        .trim_start_matches(FRONT_MATTER_DELIMITER)
        .trim_end_matches(FRONT_MATTER_DELIMITER);

    toml::from_str::<FrontMatter>(trimmed_content).map_err(|source| {
        // Calculate the proper span based on the TOML error location
        let span = if let Some(span) = source.span() {
            // Add offset for the frontmatter delimiter and any initial whitespace
            let delimiter_offset = content.find("+++").unwrap_or(0) + 3;
            let content_start = content[delimiter_offset..]
                .find(|char: char| !char.is_whitespace())
                .unwrap_or(0);
            let actual_offset = delimiter_offset + content_start;
            SourceSpan::from((actual_offset + span.start, span.len()))
        } else {
            // Fall back to highlighting the entire frontmatter content
            SourceSpan::from((0, content.len()))
        };

        TobogganCliError::parse_frontmatter(
            file_path,
            content.to_string(), // Use original content with delimiters for context
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

pub(super) fn process_talk_metadata(
    toboggan_dir: &TobogganDir,
    theme: &str,
) -> Result<TalkMetadata> {
    let mut metadata = TalkMetadata::default();

    if let Some(cover) = toboggan_dir.get_cover()? {
        let path = cover.path();
        debug!("Processing cover slide: {}", path.display());
        let (cover_slide, front_matter) = create_slide_from_file(&path, theme)?;
        metadata.title = cover_slide.title.to_string();
        metadata.date = front_matter
            .date
            .and_then(|date| parse_date_string(&date).ok())
            .unwrap_or_else(Date::today);
    }

    if let Some(footer) = toboggan_dir.get_footer()? {
        let path = footer.path();
        debug!("Processing footer: {}", path.display());
        let content = std::fs::read_to_string(&path)?;
        metadata.footer = Some(content);
    }

    if let Some(head) = toboggan_dir.get_head()? {
        let path = head.path();
        debug!("Processing head: {}", path.display());
        let content = std::fs::read_to_string(&path)?;
        metadata.head = Some(content);
    }

    Ok(metadata)
}

pub(super) fn process_all_entries(
    toboggan_dir: &TobogganDir,
    theme: &str,
) -> Result<Vec<SlideProcessingResult>> {
    let mut result = vec![];

    // Process cover slide first if it exists
    if let Some(cover) = toboggan_dir.get_cover()? {
        let path = cover.path();
        debug!("Processing cover slide: {}", path.display());
        let slide_result = process_single_file(&path, theme);
        result.push(slide_result);
    }

    let entries = toboggan_dir.get_processable_entries()?;

    for entry in entries {
        let path = entry.path();

        if path.is_dir() {
            let folder_results = process_folder_comprehensive(&path, theme)?;
            result.extend(folder_results);
        } else if is_slide_file(&path) {
            debug!("Processing file as slide: {}", path.display());
            let slide_result = process_single_file(&path, theme);
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

fn process_single_file(path: &Path, theme: &str) -> SlideProcessingResult {
    match create_slide_from_file(path, theme) {
        Ok((slide, front_matter)) => {
            if front_matter.skip {
                SlideProcessingResult::Skipped(slide)
            } else {
                SlideProcessingResult::Processed(slide)
            }
        }
        Err(error) => {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown file");
            SlideProcessingResult::Error(format!("Error processing {filename}: {error}"))
        }
    }
}

fn process_folder_comprehensive(folder: &Path, theme: &str) -> Result<Vec<SlideProcessingResult>> {
    let mut results = vec![];
    debug!("Processing folder as part: {}", folder.display());

    let toboggan_dir = TobogganDir::new(folder.to_path_buf())?;

    // Process part slide if it exists
    if let Some(part_entry) = toboggan_dir.get_part()? {
        let path = part_entry.path();
        let part_result = process_single_file(&path, theme);
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
        results.push(process_single_file(&path, theme));
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
        assert!(filenames.contains(&"slide1.md".to_string()));
        assert!(filenames.contains(&"slide2.md".to_string()));
        assert!(!filenames.contains(&"_cover.md".to_string()));
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
}
