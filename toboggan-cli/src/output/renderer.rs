//! Common traits and error handling for output renderers

use toboggan_core::Talk;

use crate::error::Result;

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum RenderError {
    #[display("Serialization failed: {message}")]
    SerializationFailed { message: String },

    #[display("Unsupported format: {format}")]
    UnsupportedFormat { format: String },

    #[display("Output too large: {size} bytes")]
    OutputTooLarge { size: usize },
}

pub trait OutputRenderer {
    type Output;

    /// Render a talk to the output format
    ///
    /// # Errors
    /// Returns an error if serialization fails
    fn render(&self, talk: &Talk) -> Result<Self::Output>;

    /// Get the MIME type for this format
    fn mime_type(&self) -> &'static str;

    /// Get the file extension for this format
    fn extension(&self) -> &'static str;

    /// Check if this format is binary
    fn is_binary(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct RenderMetrics {
    pub input_size: usize,
    pub output_size: usize,
    pub compression_ratio: f64,
    pub render_time_ms: u64,
}

impl RenderMetrics {
    #[allow(clippy::cast_precision_loss)]
    pub fn new(input_size: usize, output_size: usize, render_time_ms: u64) -> Self {
        let compression_ratio = if input_size > 0 {
            output_size as f64 / input_size as f64
        } else {
            1.0
        };

        Self {
            input_size,
            output_size,
            compression_ratio,
            render_time_ms,
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
pub fn measure_render<F, T>(func: F) -> (T, u64)
where
    F: FnOnce() -> T,
{
    let start = std::time::Instant::now();
    let result = func();
    let duration = start.elapsed().as_millis() as u64;
    (result, duration)
}
