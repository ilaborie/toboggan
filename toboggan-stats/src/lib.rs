//! Statistics computation for Toboggan presentations.
//!
//! This crate provides utilities for analyzing presentation content:
//! - Word counting
//! - Bullet point counting (via HTML `<li>` elements)
//! - Image counting (`img`, `svg`, `figure` elements)
//! - Animation step counting (`.step` CSS class)
//! - Duration estimation at different speaking rates

mod analysis;
mod html;
mod slide_stats;

pub use analysis::{
    count_images_in_html, count_list_items_in_html, count_steps_from_content, count_words,
    extract_text_from_html, strip_markdown_link_urls, strip_slide_counter,
};
pub use html::HtmlDocument;
pub use slide_stats::{DurationEstimate, PresentationStats, SlideStats};
