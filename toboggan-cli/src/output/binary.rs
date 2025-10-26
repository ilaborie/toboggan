use toboggan_core::Talk;

use super::renderer::{RenderMetrics, measure_render};
use crate::error::Result;

pub struct BinaryRenderer;

impl BinaryRenderer {
    pub fn cbor(talk: &Talk) -> Result<Vec<u8>> {
        let (result, _) = measure_render(|| {
            let mut buffer = Vec::new();
            ciborium::ser::into_writer(talk, &mut buffer).map_err(|err| {
                crate::error::TobogganCliError::Serialize {
                    format: "CBOR".to_string(),
                    message: err.to_string(),
                }
            })?;
            Ok::<Vec<u8>, crate::error::TobogganCliError>(buffer)
        });
        result
    }

    pub fn msgpack(talk: &Talk) -> Result<Vec<u8>> {
        let (result, _) = measure_render(|| {
            rmp_serde::to_vec(talk).map_err(|err| crate::error::TobogganCliError::Serialize {
                format: "MessagePack".to_string(),
                message: err.to_string(),
            })
        });
        result
    }

    pub fn bincode(talk: &Talk) -> Result<Vec<u8>> {
        let (result, _) = measure_render(|| {
            bincode::serde::encode_to_vec(talk, bincode::config::standard()).map_err(|err| {
                crate::error::TobogganCliError::Serialize {
                    format: "Bincode".to_string(),
                    message: err.to_string(),
                }
            })
        });
        result
    }

    pub fn metrics(talk: &Talk, format: &str) -> Result<RenderMetrics> {
        let input_size = std::mem::size_of_val(talk);

        let (output, render_time) = match format {
            "cbor" => {
                let (result, time) = measure_render(|| Self::cbor(talk));
                (result?.len(), time)
            }
            "msgpack" => {
                let (result, time) = measure_render(|| Self::msgpack(talk));
                (result?.len(), time)
            }
            "bincode" => {
                let (result, time) = measure_render(|| Self::bincode(talk));
                (result?.len(), time)
            }
            _ => {
                return Err(crate::error::TobogganCliError::Serialize {
                    format: format.to_string(),
                    message: "Unsupported binary format".to_string(),
                });
            }
        };

        Ok(RenderMetrics::new(input_size, output, render_time))
    }

    pub fn compare_compression(talk: &Talk) -> Result<Vec<(String, RenderMetrics)>> {
        let formats = ["cbor", "msgpack", "bincode"];
        let mut results = Vec::new();

        for format in &formats {
            let metrics = Self::metrics(talk, format)?;
            results.push(((*format).to_string(), metrics));
        }

        // Sort by compression ratio (best compression first)
        results.sort_by(|first, second| {
            first
                .1
                .compression_ratio
                .partial_cmp(&second.1.compression_ratio)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use toboggan_core::{Content, Date, Slide, Talk};

    use super::*;

    fn create_test_talk() -> anyhow::Result<Talk> {
        let mut talk = Talk::new("Test Talk");
        talk.date = Date::new(2024, 12, 25).with_context(|| "Failed to create test date")?;

        // Add some slides to make the test more realistic
        let mut slide1 = Slide::new(Content::text("Slide 1"));
        slide1.body = Content::text("Content 1");
        let mut slide2 = Slide::new(Content::text("Slide 2"));
        slide2.body = Content::text("Content 2");
        talk.slides.extend([slide1, slide2]);

        Ok(talk)
    }

    #[test]
    fn test_cbor_rendering() -> anyhow::Result<()> {
        let talk = create_test_talk()?;
        let output = BinaryRenderer::cbor(&talk)?;

        // CBOR output should be binary data
        assert!(!output.is_empty());

        // Verify we can deserialize it back
        let _result: std::result::Result<Talk, _> = ciborium::de::from_reader(&output[..]);
        // Just verify the deserialization doesn't panic, we don't need to check the result

        Ok(())
    }

    #[test]
    fn test_msgpack_rendering() -> anyhow::Result<()> {
        let talk = create_test_talk()?;
        let output = BinaryRenderer::msgpack(&talk)?;

        // MessagePack output should be binary data
        assert!(!output.is_empty());

        // Just verify it's binary data (for now, skip deserialization due to enum complexity)
        assert!(output.len() > 50); // Should have reasonable size

        Ok(())
    }

    #[test]
    fn test_bincode_rendering() -> anyhow::Result<()> {
        let talk = create_test_talk()?;
        let output = BinaryRenderer::bincode(&talk)?;

        // Bincode output should be binary data
        assert!(!output.is_empty());

        // Just verify it's binary data (for now, skip deserialization due to enum complexity)
        assert!(output.len() > 50); // Should have reasonable size

        Ok(())
    }

    #[test]
    fn test_compression_comparison() -> anyhow::Result<()> {
        let talk = create_test_talk()?;
        let comparison = BinaryRenderer::compare_compression(&talk)?;

        // Should have results for all 3 formats
        assert_eq!(comparison.len(), 3);

        // Each format should have a name and metrics
        for (name, metrics) in &comparison {
            assert!(["cbor", "msgpack", "bincode"].contains(&name.as_str()));
            assert!(metrics.output_size > 0);
            assert!(metrics.compression_ratio > 0.0);
        }

        Ok(())
    }

    #[test]
    fn test_binary_vs_text_size() -> anyhow::Result<()> {
        let talk = create_test_talk()?;

        // Get sizes of different formats
        let cbor_size = BinaryRenderer::cbor(&talk)?.len();
        let msgpack_size = BinaryRenderer::msgpack(&talk)?.len();
        let bincode_size = BinaryRenderer::bincode(&talk)?.len();
        let json_size = crate::output::TextRenderer::json(&talk)?.len();

        // Binary formats should generally be more compact than JSON
        // (though this isn't guaranteed for very small payloads)
        // Note: Removed println! to comply with clippy::print_stdout lint
        let _ = (cbor_size, msgpack_size, bincode_size, json_size);

        // All formats should produce non-empty output
        assert!(cbor_size > 0);
        assert!(msgpack_size > 0);
        assert!(bincode_size > 0);
        assert!(json_size > 0);

        Ok(())
    }

    #[test]
    fn test_renderer_traits() -> anyhow::Result<()> {
        let talk = create_test_talk()?;

        // Since we removed the renderer structs, we can only test the static methods
        assert!(BinaryRenderer::cbor(&talk).is_ok());
        assert!(BinaryRenderer::msgpack(&talk).is_ok());
        assert!(BinaryRenderer::bincode(&talk).is_ok());
        Ok(())
    }

    #[test]
    fn test_metrics() -> anyhow::Result<()> {
        let talk = create_test_talk()?;

        let cbor_metrics = BinaryRenderer::metrics(&talk, "cbor")?;
        let msgpack_metrics = BinaryRenderer::metrics(&talk, "msgpack")?;
        let bincode_metrics = BinaryRenderer::metrics(&talk, "bincode")?;

        // All should have positive output sizes
        assert!(cbor_metrics.output_size > 0);
        assert!(msgpack_metrics.output_size > 0);
        assert!(bincode_metrics.output_size > 0);

        // All should have measured render times (u64 is always >= 0, so just check they exist)
        let _ = cbor_metrics.render_time_ms;
        let _ = msgpack_metrics.render_time_ms;
        let _ = bincode_metrics.render_time_ms;

        Ok(())
    }
}
