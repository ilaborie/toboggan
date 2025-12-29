use toboggan_core::{Slide as CoreSlide, SlideKind as CoreSlideKind};
use toboggan_stats::SlideStats;

/// A slide kind
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum SlideKind {
    /// Cover
    Cover,
    /// Part header
    Part,
    /// Standard
    Standard,
}

/// A slide
#[derive(Debug, Clone, uniffi::Record)]
pub struct Slide {
    pub title: String,
    pub kind: SlideKind,
    pub step_count: u32,
}

impl From<CoreSlide> for Slide {
    fn from(value: CoreSlide) -> Self {
        // Compute step count using toboggan-stats
        let step_count = SlideStats::from_slide(&value).steps;

        #[allow(clippy::cast_possible_truncation)]
        // UniFFI requires u32, step counts are typically small
        Self {
            title: value.title.to_string(),
            kind: match value.kind {
                CoreSlideKind::Cover => SlideKind::Cover,
                CoreSlideKind::Part => SlideKind::Part,
                CoreSlideKind::Standard => SlideKind::Standard,
            },
            step_count: step_count as u32,
        }
    }
}
