use toboggan_core::{Slide as CoreSlide, SlideKind as CoreSlideKind};

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
        let CoreSlide {
            kind,
            title,
            step_count,
            ..
        } = value;
        #[allow(clippy::cast_possible_truncation)]
        // UniFFI requires u32, step counts are typically small
        Self {
            title: title.to_string(),
            kind: match kind {
                CoreSlideKind::Cover => SlideKind::Cover,
                CoreSlideKind::Part => SlideKind::Part,
                CoreSlideKind::Standard => SlideKind::Standard,
            },
            step_count: step_count as u32,
        }
    }
}
