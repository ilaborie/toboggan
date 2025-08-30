//! Folder-based presentation parser implementation

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use toboggan_core::{Content, Date, Slide, SlideKind, Talk};
use tracing::debug;

use super::{Parser, utils};
use crate::parse_date_string;

/// Parser for folder-based presentations
pub struct FolderParser {
    path: Box<Path>,
}

impl FolderParser {
    /// Create a new folder parser from a path
    #[must_use]
    pub fn new(path: &Path) -> Self {
        Self { path: path.into() }
    }

    fn scan_directory(&self) -> Result<Vec<fs::DirEntry>> {
        debug!("Scanning folder structure in {}", self.path.display());

        let mut entries: Vec<_> = fs::read_dir(&*self.path)
            .with_context(|| format!("reading directory {}", self.path.display()))?
            .collect::<Result<Vec<_>, _>>()?;

        entries.sort_by_key(fs::DirEntry::file_name);
        Ok(entries)
    }

    fn extract_metadata(
        &self,
        title_override: Option<Content>,
        date_override: Option<Date>,
    ) -> (Content, Date, Content) {
        let title = title_override
            .or_else(|| self.find_title())
            .unwrap_or_else(|| {
                let folder_name = self
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Untitled Talk");
                Content::from(folder_name)
            });

        let date = date_override
            .or_else(|| self.find_date())
            .unwrap_or_else(Date::today);

        let footer = self.find_footer().unwrap_or_default();

        (title, date, footer)
    }

    fn find_title(&self) -> Option<Content> {
        let title_md = self.path.join("title.md");
        let title_txt = self.path.join("title.txt");

        if let Ok(content) = fs::read_to_string(&title_md) {
            return Some(Content::from(content.trim()));
        }

        if let Ok(content) = fs::read_to_string(&title_txt) {
            return Some(Content::from(content.trim()));
        }

        None
    }

    fn find_date(&self) -> Option<Date> {
        let date_file = self.path.join("date.txt");
        if let Ok(date_str) = fs::read_to_string(&date_file)
            && let Ok(date) = parse_date_string(date_str.trim())
        {
            return Some(date);
        }
        None
    }

    fn find_footer(&self) -> Option<Content> {
        let footer_file = self.path.join("_footer.md");

        if let Ok(content) = fs::read_to_string(&footer_file) {
            let content = content.trim();
            if !content.is_empty() {
                return Some(utils::markdown_to_html_content(content));
            }
        }

        None
    }
}

impl Parser for FolderParser {
    fn parse(&self, title_override: Option<Content>, date_override: Option<Date>) -> Result<Talk> {
        let entries = self.scan_directory()?;
        let (title, date, footer) = self.extract_metadata(title_override, date_override);

        let mut talk = Talk::new(title).with_date(date).with_footer(footer);
        let mut processor = SlideProcessor::new(&mut talk);

        processor.process_cover(&entries)?;
        processor.process_entries(entries)?;

        Ok(talk)
    }
}

struct SlideProcessor<'a> {
    talk: &'a mut Talk,
}

impl<'a> SlideProcessor<'a> {
    fn new(talk: &'a mut Talk) -> Self {
        Self { talk }
    }

    fn process_cover(&mut self, entries: &[fs::DirEntry]) -> Result<()> {
        for entry in entries {
            let path = entry.path();
            let filename = entry.file_name();

            if filename.to_string_lossy() == "_cover.md" && path.is_file() {
                debug!("Processing cover slide: {}", path.display());
                let cover_slide = utils::create_slide_from_file(&path)?;
                *self.talk = self.talk.clone().add_slide(cover_slide);
                break;
            }
        }
        Ok(())
    }

    fn process_entries(&mut self, entries: Vec<fs::DirEntry>) -> Result<()> {
        for entry in entries {
            let path = entry.path();
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            if should_skip_entry(&filename_str) {
                continue;
            }

            if path.is_dir() {
                self.process_folder(&path)?;
            } else if utils::is_slide_file(&path) {
                debug!("Processing file as slide: {}", path.display());
                let slide = utils::create_slide_from_file(&path)?;
                *self.talk = self.talk.clone().add_slide(slide);
            }
        }
        Ok(())
    }

    fn process_folder(&mut self, folder: &Path) -> Result<()> {
        debug!("Processing folder as part: {}", folder.display());

        let part_slide = create_part_slide(folder)?;
        *self.talk = self.talk.clone().add_slide(part_slide);

        let folder_slides = parse_folder_contents(folder)?;
        for slide in folder_slides {
            *self.talk = self.talk.clone().add_slide(slide);
        }

        Ok(())
    }
}

fn should_skip_entry(filename: &str) -> bool {
    filename.starts_with('.')
        || filename == "title.md"
        || filename == "title.txt"
        || filename == "date.txt"
        || filename == "_cover.md"
        || filename == "_footer.md"
}

fn create_part_slide(folder: &Path) -> Result<Slide> {
    let part_md = folder.join("_part.md");

    if part_md.exists() {
        let content = fs::read_to_string(&part_md)
            .with_context(|| format!("reading {}", part_md.display()))?;
        let slide = utils::parse_slide_from_markdown(&content, SlideKind::Part);
        Ok(slide)
    } else {
        let folder_name = folder
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled Part");
        Ok(Slide::part(folder_name))
    }
}

fn parse_folder_contents(folder: &Path) -> Result<Vec<Slide>> {
    let mut slides = Vec::new();

    let mut entries: Vec<_> = fs::read_dir(folder)
        .with_context(|| format!("reading directory {}", folder.display()))?
        .collect::<Result<Vec<_>, _>>()?;

    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        if filename_str.starts_with('.') || filename_str.starts_with('_') {
            continue;
        }

        if path.is_file() && utils::is_slide_file(&path) {
            debug!("Processing folder content file: {}", path.display());
            let slide = utils::create_slide_from_file(&path)?;
            slides.push(slide);
        }
    }

    Ok(slides)
}
