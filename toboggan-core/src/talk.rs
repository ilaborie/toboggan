//! Talk definitions and builder patterns.
//!
//! This module provides the [`Talk`] struct for representing complete
//! presentations with metadata and slides.

use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::{Content, Date, Slide};

/// A complete presentation with metadata and slides.
///
/// A `Talk` represents a full presentation, including its title, date,
/// and a collection of slides. It serves as the top-level container
/// for all presentation content.
///
/// # Examples
///
/// ```rust
/// use toboggan_core::{Talk, Slide, date_utils};
///
/// let talk = Talk::new("Rust Programming")
///     .with_date(date_utils::ymd(2025, 1, 26))
///     .add_slide(Slide::cover("Introduction"))
///     .add_slide(Slide::new("Getting Started"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Talk {
    /// The title of the presentation.
    pub title: Content,

    /// The date of the presentation.
    pub date: Date,

    /// The slides that make up the presentation.
    pub slides: Vec<Slide>,
}

impl Talk {
    /// Creates a new talk with the given title.
    ///
    /// The date is set to today and the slides collection is empty.
    /// Use the builder methods to add slides and set a specific date.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::Talk;
    ///
    /// let talk = Talk::new("My Presentation");
    /// let talk2 = Talk::new("Advanced Topics");
    /// ```
    pub fn new(title: impl Into<Content>) -> Self {
        let title = title.into();
        let date = Date::today();
        let slides = Vec::new();

        Self {
            title,
            date,
            slides,
        }
    }

    /// Sets the date of the presentation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::{Talk, date_utils};
    ///
    /// let talk = Talk::new("Conference Talk")
    ///     .with_date(date_utils::ymd(2025, 3, 15));
    /// ```
    #[must_use]
    pub fn with_date(mut self, date: Date) -> Self {
        self.date = date;
        self
    }

    /// Adds a slide to the presentation.
    ///
    /// Slides are added in the order they will be presented.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toboggan_core::{Talk, Slide};
    ///
    /// let talk = Talk::new("My Talk")
    ///     .add_slide(Slide::cover("Welcome"))
    ///     .add_slide(Slide::new("Overview"))
    ///     .add_slide(Slide::part("Chapter 1"))
    ///     .add_slide(Slide::new("Introduction"));
    /// ```
    #[must_use]
    pub fn add_slide(mut self, slide: Slide) -> Self {
        self.slides.push(slide);
        self
    }
}
