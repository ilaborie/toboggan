use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

use comrak::arena_tree::Node;
use comrak::nodes::Ast;
use serde::{Deserialize, Deserializer, Serialize};
use toboggan_core::{Date, Style};

use crate::ParseResult;
use crate::error::Result;

mod content;
pub use self::content::SlideContentParser;

mod renderer;
use self::renderer::{ContentRenderer, HtmlRenderer};

mod config;
use self::config::{create_syntax_highlighter, default_options, default_plugins};

mod comments;
mod directory;
use self::directory::{TobogganDir, process_all_entries, process_talk_metadata};

type MarkdownNode<'a> = Node<'a, RefCell<Ast>>;

type CssClasses = Vec<String>;

const FRONT_MATTER_DELIMITER: &str = "+++";

const DEFAULT_SLIDE_TITLE: &str = "<No Title>";
const DEFAULT_PART_TITLE: &str = "Untitled Part";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub skip: bool,
    pub date: Option<String>,
    pub classes: CssClasses,
    pub grid: bool,
    pub style: Option<String>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_duration"
    )]
    pub duration: Option<Duration>,
}

impl FrontMatter {
    pub fn to_style(&self) -> Result<Style> {
        Ok(Style {
            classes: self.classes.clone(),
            style: self.style.clone(),
        })
    }
}

pub struct FolderParser {
    toboggan_dir: TobogganDir,
    theme: String,
}

impl FolderParser {
    pub fn new(path: PathBuf, theme: String) -> Result<Self> {
        let toboggan_dir = TobogganDir::new(path)?;
        Ok(Self {
            toboggan_dir,
            theme,
        })
    }

    pub fn parse(
        &self,
        title_override: Option<String>,
        date_override: Option<Date>,
    ) -> Result<ParseResult> {
        let mut talk_metadata = process_talk_metadata(&self.toboggan_dir, &self.theme)?;
        if let Some(title) = title_override {
            talk_metadata.title = title;
        }
        if let Some(date) = date_override {
            talk_metadata.date = date;
        }

        let slides = process_all_entries(&self.toboggan_dir, &self.theme)?;

        Ok(ParseResult {
            talk_metadata,
            slides,
        })
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::TobogganCliError;
    use crate::parser::directory::create_test_file;

    #[test]
    fn test_folder_parser_basic() -> Result<()> {
        let temp_dir = tempdir()?;
        let dir_path = temp_dir.path();

        // Create basic folder structure
        create_test_file(
            dir_path,
            "_cover.md",
            "# Test Presentation\n\nThis is a test.",
        )?;
        create_test_file(dir_path, "slide1.md", "# First Slide\n\nContent here.")?;

        let parser = FolderParser::new(dir_path.to_path_buf(), "base16-ocean.light".to_string())?;
        let result = parser.parse(None, None)?;
        let talk = result.to_talk();

        assert_eq!(result.talk_metadata.title, "Test Presentation");
        assert!(!talk.slides.is_empty());

        Ok(())
    }

    #[test]
    fn test_folder_parser_with_part() -> Result<()> {
        let temp_dir = tempdir()?;
        let dir_path = temp_dir.path();

        // Create folder with part
        let part_dir = dir_path.join("01-intro");
        fs::create_dir(&part_dir)?;

        create_test_file(&part_dir, "_part.md", "# Introduction")?;
        create_test_file(&part_dir, "slide1.md", "# Content Slide")?;

        let parser = FolderParser::new(dir_path.to_path_buf(), "base16-ocean.light".to_string())?;
        let result = parser.parse(None, None)?;
        let talk = result.to_talk();

        // Should have part slide + content slide
        assert!(talk.slides.len() >= 2);

        Ok(())
    }

    #[test]
    fn test_folder_parser_with_overrides() -> Result<()> {
        let temp_dir = tempdir()?;
        let dir_path = temp_dir.path();

        create_test_file(dir_path, "_cover.md", "# Original Title")?;

        let parser = FolderParser::new(dir_path.to_path_buf(), "base16-ocean.light".to_string())?;
        let custom_date = Date::new(2024, 12, 25).map_err(|_| TobogganCliError::InvalidDate {
            year: 2024,
            month: 12,
            day: 25,
        })?;
        let result = parser.parse(Some("Override Title".to_string()), Some(custom_date))?;
        let _talk = result.to_talk();

        assert_eq!(result.talk_metadata.title, "Override Title");
        assert_eq!(result.talk_metadata.date, custom_date);

        Ok(())
    }

    #[test]
    fn test_skip_slides_functionality() -> Result<()> {
        let temp_dir = tempdir()?;
        let dir_path = temp_dir.path();

        // Create cover slide
        create_test_file(dir_path, "_cover.md", "# Test Talk")?;

        // Create regular slide
        create_test_file(
            dir_path,
            "slide1.md",
            "# Regular Slide\n\nThis should be included.",
        )?;

        // Create slide with skip: true
        create_test_file(
            dir_path,
            "slide2.md",
            "+++\nskip = true\n+++\n\n# Skipped Slide\n\nThis should not be included.",
        )?;

        // Create another regular slide
        create_test_file(
            dir_path,
            "slide3.md",
            "# Another Regular Slide\n\nThis should be included.",
        )?;

        let parser = FolderParser::new(dir_path.to_path_buf(), "base16-ocean.light".to_string())?;
        let result = parser.parse(None, None)?;
        let talk = result.to_talk();

        // Should have 3 slides (cover, slide1 and slide3, not slide2)
        assert_eq!(talk.slides.len(), 3);
        assert_eq!(
            talk.slides[0].title,
            toboggan_core::Content::text("Test Talk")
        );
        assert_eq!(
            talk.slides[1].title,
            toboggan_core::Content::text("Regular Slide")
        );
        assert_eq!(
            talk.slides[2].title,
            toboggan_core::Content::text("Another Regular Slide")
        );

        // Check that we have 4 total slide results (cover + 2 regular + 1 skipped)
        assert_eq!(result.slides.len(), 4);
        let stats = result.stats();
        assert_eq!(stats.processed, 3); // cover + slide1 + slide3
        assert_eq!(stats.skipped, 1); // slide2

        Ok(())
    }

    #[test]
    fn test_skip_part_slide() -> Result<()> {
        let temp_dir = tempdir()?;
        let dir_path = temp_dir.path();

        // Create cover slide
        create_test_file(dir_path, "_cover.md", "# Test Talk")?;

        // Create part directory with skipped part slide
        let part_dir = dir_path.join("01-intro");
        fs::create_dir(&part_dir)?;

        create_test_file(
            &part_dir,
            "_part.md",
            "+++\nskip = true\n+++\n\n# Introduction\n\nThis part should be skipped.",
        )?;
        create_test_file(
            &part_dir,
            "content.md",
            "# Content Slide\n\nThis should be included.",
        )?;

        let parser = FolderParser::new(dir_path.to_path_buf(), "base16-ocean.light".to_string())?;
        let result = parser.parse(None, None)?;
        let talk = result.to_talk();

        // Should have 2 slides (cover and content slide, not the part slide)
        assert_eq!(talk.slides.len(), 2);
        assert_eq!(
            talk.slides[0].title,
            toboggan_core::Content::text("Test Talk")
        );
        assert_eq!(
            talk.slides[1].title,
            toboggan_core::Content::text("Content Slide")
        );

        // Check that we have 3 total slide results (cover + skipped part + content)
        assert_eq!(result.slides.len(), 3);
        let stats = result.stats();
        assert_eq!(stats.processed, 2); // cover + content
        assert_eq!(stats.skipped, 1); // skipped part

        Ok(())
    }

    #[test]
    fn test_part_md_appears_only_once() -> Result<()> {
        let temp_dir = tempdir()?;
        let dir_path = temp_dir.path();

        // Create cover slide
        create_test_file(dir_path, "_cover.md", "# Test Talk")?;

        // Create part directory with _part.md
        let part_dir = dir_path.join("01-intro");
        fs::create_dir(&part_dir)?;

        create_test_file(
            &part_dir,
            "_part.md",
            "# Introduction\n\nThis is the intro part.",
        )?;
        create_test_file(
            &part_dir,
            "content.md",
            "# Content Slide\n\nThis should be included.",
        )?;

        let parser = FolderParser::new(dir_path.to_path_buf(), "base16-ocean.light".to_string())?;
        let result = parser.parse(None, None)?;
        let talk = result.to_talk();

        // Should have exactly 3 slides: cover + part slide + content slide
        assert_eq!(talk.slides.len(), 3);
        assert_eq!(
            talk.slides[0].title,
            toboggan_core::Content::text("Test Talk")
        );
        assert_eq!(
            talk.slides[1].title,
            toboggan_core::Content::text("Introduction")
        );
        assert_eq!(
            talk.slides[2].title,
            toboggan_core::Content::text("Content Slide")
        );

        // Verify no duplicate "Introduction" slides
        let intro_count = talk
            .slides
            .iter()
            .filter(|slide| slide.title == toboggan_core::Content::text("Introduction"))
            .count();
        assert_eq!(intro_count, 1, "_part.md should appear only once");

        Ok(())
    }
}

fn deserialize_duration<'de, D>(deserializer: D) -> std::result::Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DurationValue {
        Seconds(u64),
        String(String),
    }

    let value = Option::<DurationValue>::deserialize(deserializer)?;

    match value {
        Some(DurationValue::Seconds(secs)) => Ok(Some(Duration::from_secs(secs))),
        Some(DurationValue::String(duration_str)) => humantime::parse_duration(&duration_str)
            .map(Some)
            .map_err(|err| {
                D::Error::custom(format!("Invalid duration format '{duration_str}': {err}"))
            }),
        None => Ok(None),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod duration_tests {
    use super::*;

    #[test]
    fn test_humantime_duration_parsing() {
        // Test humantime parsing directly
        assert_eq!(
            humantime::parse_duration("30s").expect("30s should parse"),
            Duration::from_secs(30)
        );
        assert_eq!(
            humantime::parse_duration("2m").expect("2m should parse"),
            Duration::from_secs(120)
        );
        assert_eq!(
            humantime::parse_duration("1m 30s").expect("1m 30s should parse"),
            Duration::from_secs(90)
        );
        assert_eq!(
            humantime::parse_duration("1h").expect("1h should parse"),
            Duration::from_secs(3600)
        );
        assert_eq!(
            humantime::parse_duration("1h 30m").expect("1h 30m should parse"),
            Duration::from_secs(5400)
        );
        assert_eq!(
            humantime::parse_duration("1h 30m 45s").expect("1h 30m 45s should parse"),
            Duration::from_secs(5445)
        );

        // Test additional formats supported by humantime
        assert_eq!(
            humantime::parse_duration("1hour").expect("1hour should parse"),
            Duration::from_secs(3600)
        );
        assert_eq!(
            humantime::parse_duration("2 minutes").expect("2 minutes should parse"),
            Duration::from_secs(120)
        );
        assert_eq!(
            humantime::parse_duration("30 seconds").expect("30 seconds should parse"),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn test_frontmatter_duration_parsing() {
        let toml_content = r#"
title = "Test Slide"
duration = 90
"#;
        let frontmatter: FrontMatter = toml::from_str(toml_content).expect("TOML should parse");
        assert_eq!(frontmatter.duration, Some(Duration::from_secs(90)));

        let toml_content = r#"
title = "Test Slide"
duration = "2m 30s"
"#;
        let frontmatter: FrontMatter = toml::from_str(toml_content).expect("TOML should parse");
        assert_eq!(frontmatter.duration, Some(Duration::from_secs(150)));

        let toml_content = r#"
title = "Test Slide"
duration = "1 hour 30 minutes"
"#;
        let frontmatter: FrontMatter = toml::from_str(toml_content).expect("TOML should parse");
        assert_eq!(frontmatter.duration, Some(Duration::from_secs(5400)));

        let toml_content = r#"
title = "Test Slide"
"#;
        let frontmatter: FrontMatter = toml::from_str(toml_content).expect("TOML should parse");
        assert_eq!(frontmatter.duration, None);
    }
}
