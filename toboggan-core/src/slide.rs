//! Slide definitions and builder patterns.
//!
//! This module provides the [`Slide`] struct and related types for creating
//! individual slides within a presentation. Slides have different kinds
//! (Cover, Part, Standard) and support styling, content, and speaker notes.

#[cfg(feature = "utoipa")]
use alloc::format;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{Content, SlideId};

/// A single slide within a presentation.
///
/// Slides are the building blocks of a [`Talk`](crate::Talk). Each slide has a kind
/// that determines its role in the presentation, optional styling, and three content
/// areas: title, body, and notes.
///
/// # Examples
///
/// ```rust
/// use toboggan_core::{Slide, Content};
///
/// // Create a cover slide
/// let cover = Slide::cover("Welcome to Rust")
///     .with_body("An introduction to systems programming");
///
/// // Create a standard content slide
/// let content = Slide::new("Memory Safety")
///     .with_body(Content::html("<ul><li>No null pointers</li><li>No buffer overflows</li></ul>"))
///     .with_notes("Remember to mention the borrow checker");
///
/// // Create a section divider
/// let section = Slide::part("Chapter 2: Ownership");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(default)]
pub struct Slide {
    /// The slide id
    pub id: SlideId,

    /// The kind of slide (Cover, Part, or Standard).
    pub kind: SlideKind,

    /// Styling information for the slide.
    pub style: Style,

    /// The main title of the slide.
    pub title: Content,

    /// The main content body of the slide.
    pub body: Content,

    /// Speaker notes (not visible to audience).
    pub notes: Content,
}

impl Slide {
    /// Creates a new standard slide with the given title.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Slide;
    ///
    /// let slide = Slide::new("Introduction");
    /// let slide2 = Slide::new("Advanced Topics");
    /// ```
    pub fn new(title: impl Into<Content>) -> Self {
        let title = title.into();
        Self {
            title,
            ..Default::default()
        }
    }

    /// Creates a cover slide with the given title.
    ///
    /// Cover slides are typically used for the presentation title page
    /// and have special styling in most themes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Slide;
    ///
    /// let cover = Slide::cover("Rust Programming");
    /// ```
    pub fn cover(title: impl Into<Content>) -> Self {
        let title = title.into();
        Self {
            kind: SlideKind::Cover,
            title,
            ..Default::default()
        }
    }

    /// Creates a part slide with the given title.
    ///
    /// Part slides are typically used as section dividers within
    /// a presentation and have distinctive styling.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Slide;
    ///
    /// let part = Slide::part("Chapter 1: Getting Started");
    /// ```
    pub fn part(title: impl Into<Content>) -> Self {
        let title = title.into();
        Self {
            kind: SlideKind::Part,
            title,
            ..Default::default()
        }
    }

    /// Adds style classes to the slide (only available with `alloc` feature).
    ///
    /// Styles are applied as CSS classes and can be used by themes
    /// to customize the appearance of individual slides.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "alloc")]
    /// # {
    /// use toboggan_core::Slide;
    ///
    /// let slide = Slide::new("Styled Slide")
    ///     .with_style(["large-text".to_string(), "blue-background".to_string()]);
    /// # }
    /// ```
    #[cfg(feature = "alloc")]
    #[must_use]
    pub fn with_style(mut self, styles: impl IntoIterator<Item = String>) -> Self {
        let style = Style(Vec::from_iter(styles));
        self.style = style;
        self
    }

    /// Sets the body content of the slide.
    ///
    /// The body is the main content area of the slide and can contain
    /// any type of [`Content`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::{Slide, Content};
    ///
    /// let slide = Slide::new("Lists")
    ///     .with_body("Here are the key points:")
    ///     .with_body(Content::html("<ul><li>Point 1</li><li>Point 2</li></ul>"));
    /// ```
    #[must_use]
    pub fn with_body(mut self, body: impl Into<Content>) -> Self {
        self.body = body.into();
        self
    }

    /// Sets the speaker notes for the slide.
    ///
    /// Notes are only visible to the presenter and can contain
    /// reminders, timing information, or additional context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Slide;
    ///
    /// let slide = Slide::new("Complex Topic")
    ///     .with_body("This is a complex concept...")
    ///     .with_notes("Take 2 minutes to explain this. Ask for questions.");
    /// ```
    #[must_use]
    pub fn with_notes(mut self, notes: impl Into<Content>) -> Self {
        self.notes = notes.into();
        self
    }
}

/// The kind of slide, which affects its styling and role in the presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum SlideKind {
    /// Cover slide, typically used for the presentation title page.
    Cover,

    /// Part slide, used as section dividers within the presentation.
    Part,

    /// Standard content slide (default).
    #[default]
    Standard,
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

/// Styling information for slides (only available with `alloc` feature).
///
/// Contains a list of CSS class names that can be applied to customize
/// the appearance of individual slides.
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Style(Vec<String>);

/// Minimal style placeholder for `no_std` environments without `alloc`.
#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Style;

#[cfg(feature = "alloc")]
impl Display for Style {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let classes = self.0.join(" ");
        write!(fmt, "{classes}")
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SlidesResponse {
    pub slides: Vec<Slide>,
}
