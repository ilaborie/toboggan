mod renderer;
pub use self::renderer::{OutputRenderer, RenderError};

mod text;
pub use self::text::TextRenderer;

mod html;

use toboggan_core::Talk;

use crate::error::Result;
use crate::settings::OutputFormat;

pub fn serialize_talk(talk: &Talk, format: OutputFormat) -> Result<Vec<u8>> {
    match format {
        OutputFormat::Toml => TextRenderer::toml(talk),
        OutputFormat::Json => TextRenderer::json(talk),
        OutputFormat::Yaml => TextRenderer::yaml(talk),

        OutputFormat::Html => html::generate_html(talk, talk.head.as_deref()),
    }
}

#[must_use]
pub fn get_extension(format: &OutputFormat) -> &'static str {
    match format {
        OutputFormat::Toml => "toml",
        OutputFormat::Json => "json",
        OutputFormat::Yaml => "yaml",
        OutputFormat::Html => "html",
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use toboggan_core::{Date, Talk};

    use super::*;

    fn create_test_talk() -> anyhow::Result<Talk> {
        let mut talk = Talk::new("Test Talk");
        talk.date = Date::new(2024, 12, 25).with_context(|| "Failed to create test date")?;
        Ok(talk)
    }

    #[test]
    fn test_all_formats_serialize() -> anyhow::Result<()> {
        let talk = create_test_talk()?;

        // Test all formats can serialize without error
        let formats = [
            OutputFormat::Toml,
            OutputFormat::Json,
            OutputFormat::Yaml,
            OutputFormat::Html,
        ];

        for format in &formats {
            let result = serialize_talk(&talk, *format);
            assert!(result.is_ok(), "Failed to serialize format: {format:?}");
            if let Ok(output) = result {
                assert!(!output.is_empty(), "Empty output for format: {format:?}");
            }
        }
        Ok(())
    }

    #[test]
    fn test_extensions() {
        assert_eq!(get_extension(&OutputFormat::Toml), "toml");
        assert_eq!(get_extension(&OutputFormat::Json), "json");
        assert_eq!(get_extension(&OutputFormat::Yaml), "yaml");
        assert_eq!(get_extension(&OutputFormat::Html), "html");
    }
}
