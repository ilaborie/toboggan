use toboggan_core::{Slide as CoreSlide, SlideKind as CoreSlideKind};

/// A slide id
pub type Id = String;

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
    pub id: Id,
    pub title: String,
    pub kind: SlideKind,
}

impl From<CoreSlide> for Slide {
    fn from(value: CoreSlide) -> Self {
        let CoreSlide {
            id, kind, title, ..
        } = value;
        Self {
            id: id.to_string(),
            title: title.to_string(),
            kind: match kind {
                CoreSlideKind::Cover => SlideKind::Cover,
                CoreSlideKind::Part => SlideKind::Part,
                CoreSlideKind::Standard => SlideKind::Standard,
            },
        }
    }
}
