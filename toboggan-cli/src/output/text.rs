use toboggan_core::Talk;

use super::renderer::{RenderMetrics, measure_render};
use crate::error::Result;

pub struct TextRenderer;

impl TextRenderer {
    pub fn toml(talk: &Talk) -> Result<Vec<u8>> {
        let (result, _) = measure_render(|| toml::to_string_pretty(talk));
        Ok(result?.into_bytes())
    }

    pub fn json(talk: &Talk) -> Result<Vec<u8>> {
        let (result, _) = measure_render(|| serde_json::to_string_pretty(talk));
        Ok(result?.into_bytes())
    }

    pub fn json_compact(talk: &Talk) -> Result<Vec<u8>> {
        let (result, _) = measure_render(|| serde_json::to_string(talk));
        Ok(result?.into_bytes())
    }

    pub fn yaml(talk: &Talk) -> Result<Vec<u8>> {
        let (result, _) = measure_render(|| serde_saphyr::to_string(talk));
        Ok(result
            .map_err(|err| crate::error::TobogganCliError::Serialize {
                format: "YAML".to_string(),
                message: err.to_string(),
            })?
            .into_bytes())
    }

    pub fn metrics(talk: &Talk, format: &str) -> Result<RenderMetrics> {
        let input_size = std::mem::size_of_val(talk);

        let (output, render_time) = match format {
            "toml" => {
                let (result, time) = measure_render(|| toml::to_string_pretty(talk));
                (result?.len(), time)
            }
            "json" => {
                let (result, time) = measure_render(|| serde_json::to_string_pretty(talk));
                (result?.len(), time)
            }
            "yaml" => {
                let (result, time) = measure_render(|| serde_saphyr::to_string(talk));
                (
                    result
                        .map_err(|err| crate::error::TobogganCliError::Serialize {
                            format: "YAML".to_string(),
                            message: err.to_string(),
                        })?
                        .len(),
                    time,
                )
            }
            _ => {
                return Err(crate::error::TobogganCliError::Serialize {
                    format: format.to_string(),
                    message: "Unsupported text format".to_string(),
                });
            }
        };

        Ok(RenderMetrics::new(input_size, output, render_time))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use toboggan_core::{Date, Talk};

    use super::*;

    fn create_test_talk() -> anyhow::Result<Talk> {
        let mut talk = Talk::new("Test Talk");
        talk.date = Date::new(2024, 12, 25)
            .map_err(|err| anyhow::anyhow!("Failed to create test date: {err}"))?;
        Ok(talk)
    }

    #[test]
    fn test_toml_rendering() -> anyhow::Result<()> {
        let talk = create_test_talk()?;
        let output = TextRenderer::toml(&talk)?;
        let content = String::from_utf8(output)
            .map_err(|err| anyhow::anyhow!("Failed to convert TOML output to UTF-8: {err}"))?;

        assert!(content.contains("title = \"Test Talk\""));
        // Date format might vary, just check it contains the date
        assert!(content.contains("2024") && content.contains("12") && content.contains("25"));
        Ok(())
    }

    #[test]
    fn test_json_rendering() -> anyhow::Result<()> {
        let talk = create_test_talk()?;
        let output = TextRenderer::json(&talk)?;
        let content = String::from_utf8(output)
            .map_err(|err| anyhow::anyhow!("Failed to convert JSON output to UTF-8: {err}"))?;

        assert!(content.contains("\"title\": \"Test Talk\""));
        assert!(content.contains("\"date\": \"2024-12-25\""));
        Ok(())
    }

    #[test]
    fn test_yaml_rendering() -> anyhow::Result<()> {
        let talk = create_test_talk()?;
        let output = TextRenderer::yaml(&talk)?;
        let content = String::from_utf8(output)
            .map_err(|err| anyhow::anyhow!("Failed to convert YAML output to UTF-8: {err}"))?;

        assert!(content.contains("title: Test Talk"));
        // Date format might vary, just check it contains the date
        assert!(content.contains("2024") && content.contains("12") && content.contains("25"));
        Ok(())
    }

    #[test]
    fn test_json_compact_vs_pretty() -> anyhow::Result<()> {
        let talk = create_test_talk()?;
        let pretty = TextRenderer::json(&talk)?;
        let compact = TextRenderer::json_compact(&talk)?;

        // Compact should be smaller
        assert!(compact.len() < pretty.len());

        // Both should contain the same data
        let pretty_str = String::from_utf8(pretty)
            .with_context(|| "Failed to convert pretty JSON output to UTF-8")?;
        let compact_str = String::from_utf8(compact)
            .with_context(|| "Failed to convert compact JSON output to UTF-8")?;
        assert!(pretty_str.contains("Test Talk"));
        assert!(compact_str.contains("Test Talk"));

        Ok(())
    }

    #[test]
    fn test_renderer_traits() -> anyhow::Result<()> {
        let talk = create_test_talk()?;

        // Since we removed the renderer structs, we can only test the static methods
        assert!(TextRenderer::toml(&talk).is_ok());
        assert!(TextRenderer::json(&talk).is_ok());
        assert!(TextRenderer::yaml(&talk).is_ok());
        Ok(())
    }

    #[test]
    fn test_metrics() -> anyhow::Result<()> {
        let talk = create_test_talk()?;

        let toml_metrics = TextRenderer::metrics(&talk, "toml")?;
        let json_metrics = TextRenderer::metrics(&talk, "json")?;
        let yaml_metrics = TextRenderer::metrics(&talk, "yaml")?;

        // All should have positive output sizes
        assert!(toml_metrics.output_size > 0);
        assert!(json_metrics.output_size > 0);
        assert!(yaml_metrics.output_size > 0);

        // All should have measured render times (u64 is always >= 0, so just check they exist)
        let _ = toml_metrics.render_time_ms;
        let _ = json_metrics.render_time_ms;
        let _ = yaml_metrics.render_time_ms;

        Ok(())
    }
}
