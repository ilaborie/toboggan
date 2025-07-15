use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::Content;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(default)]
pub struct Slide {
    pub kind: SlideKind,
    #[serde(skip_serializing_if = "Style::is_default")]
    pub style: Style,
    #[serde(skip_serializing_if = "Content::is_empty")]
    pub title: Content,
    pub body: Content,
    #[serde(skip_serializing_if = "Content::is_empty")]
    pub notes: Content,
}

impl Slide {
    pub fn new(title: impl Into<Content>) -> Self {
        let title = title.into();
        Self {
            title,
            ..Default::default()
        }
    }

    pub fn cover(title: impl Into<Content>) -> Self {
        let title = title.into();
        Self {
            kind: SlideKind::Cover,
            title,
            ..Default::default()
        }
    }

    pub fn part(title: impl Into<Content>) -> Self {
        let title = title.into();
        Self {
            kind: SlideKind::Part,
            title,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_style_classes(mut self, classes: impl IntoIterator<Item = String>) -> Self {
        self.style.classes = Vec::from_iter(classes);
        self
    }

    #[must_use]
    pub fn with_body(mut self, body: impl Into<Content>) -> Self {
        self.body = body.into();
        self
    }

    #[must_use]
    pub fn with_notes(mut self, notes: impl Into<Content>) -> Self {
        self.notes = notes.into();
        self
    }
}

impl Display for Slide {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Content::Empty = self.title {
            write!(fmt, "{}", self.body)
        } else {
            write!(fmt, "{}", self.title)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum SlideKind {
    Cover,
    Part,
    #[default]
    Standard,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Style {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

impl Style {
    pub(crate) fn is_default(&self) -> bool {
        self.classes.is_empty() && self.style.is_none()
    }
}

impl Display for Style {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let classes = self.classes.join(" ");
        write!(fmt, "{classes}")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SlidesResponse {
    pub slides: Vec<Slide>,
}
