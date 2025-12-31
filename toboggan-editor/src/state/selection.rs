//! Selection state for the editor

use serde::Deserialize;

use super::{PartState, SlideState, TalkState};

/// Represents what is currently selected in the editor
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub enum Selection {
    #[default]
    None,
    Cover,
    Part {
        index: usize,
    },
    Slide {
        part_index: Option<usize>,
        slide_index: usize,
    },
}

impl Selection {
    /// Get the display number (1-indexed) for status bar
    #[must_use]
    pub fn display_number(&self, talk: &TalkState) -> Option<usize> {
        match self {
            Selection::None => None,
            Selection::Cover => Some(1),
            Selection::Part { index } => {
                let cover = usize::from(talk.cover.is_some());
                let loose = talk.loose_slides.len();
                let prev_parts: usize = talk
                    .parts
                    .iter()
                    .take(*index)
                    .map(|part| 1 + part.slides.len())
                    .sum();
                Some(cover + loose + prev_parts + 1)
            }
            Selection::Slide {
                part_index,
                slide_index,
            } => {
                let cover = usize::from(talk.cover.is_some());
                let loose = talk.loose_slides.len();

                if let Some(part_idx) = part_index {
                    let prev_parts: usize = talk
                        .parts
                        .iter()
                        .take(*part_idx)
                        .map(|part| 1 + part.slides.len())
                        .sum();
                    Some(cover + loose + prev_parts + 1 + slide_index + 1)
                } else {
                    Some(cover + slide_index + 1)
                }
            }
        }
    }

    /// Get the selected slide reference
    #[must_use]
    pub fn get_slide<'a>(&self, talk: &'a TalkState) -> Option<&'a SlideState> {
        match self {
            Selection::None | Selection::Part { .. } => None,
            Selection::Cover => talk.cover.as_ref(),
            Selection::Slide {
                part_index: Some(part_idx),
                slide_index,
            } => talk.parts.get(*part_idx)?.slides.get(*slide_index),
            Selection::Slide {
                part_index: None,
                slide_index,
            } => talk.loose_slides.get(*slide_index),
        }
    }

    /// Get the selected slide mutably
    pub fn get_slide_mut<'a>(&self, talk: &'a mut TalkState) -> Option<&'a mut SlideState> {
        match self {
            Selection::None | Selection::Part { .. } => None,
            Selection::Cover => talk.cover.as_mut(),
            Selection::Slide {
                part_index: Some(part_idx),
                slide_index,
            } => talk.parts.get_mut(*part_idx)?.slides.get_mut(*slide_index),
            Selection::Slide {
                part_index: None,
                slide_index,
            } => talk.loose_slides.get_mut(*slide_index),
        }
    }

    /// Get the selected part reference
    #[must_use]
    pub fn get_part<'a>(&self, talk: &'a TalkState) -> Option<&'a PartState> {
        match self {
            Selection::Part { index } => talk.parts.get(*index),
            Selection::Slide {
                part_index: Some(part_idx),
                ..
            } => talk.parts.get(*part_idx),
            _ => None,
        }
    }

    /// Get the selected part mutably
    pub fn get_part_mut<'a>(&self, talk: &'a mut TalkState) -> Option<&'a mut PartState> {
        match self {
            Selection::Part { index } => talk.parts.get_mut(*index),
            Selection::Slide {
                part_index: Some(part_idx),
                ..
            } => talk.parts.get_mut(*part_idx),
            _ => None,
        }
    }

    /// Check if this selection is a slide (not part or cover)
    #[must_use]
    pub fn is_slide(&self) -> bool {
        matches!(self, Selection::Slide { .. })
    }

    /// Check if this selection is a part
    #[must_use]
    pub fn is_part(&self) -> bool {
        matches!(self, Selection::Part { .. })
    }

    /// Check if this selection is the cover
    #[must_use]
    pub fn is_cover(&self) -> bool {
        matches!(self, Selection::Cover)
    }

    /// Check if nothing is selected
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Selection::None)
    }

    /// Get the current part index (if any)
    #[must_use]
    pub fn current_part_index(&self) -> Option<usize> {
        match self {
            Selection::Part { index } => Some(*index),
            Selection::Slide {
                part_index: Some(part_idx),
                ..
            } => Some(*part_idx),
            _ => None,
        }
    }

    /// Select cover
    #[must_use]
    pub fn select_cover() -> Self {
        Selection::Cover
    }

    /// Select a part
    #[must_use]
    pub fn select_part(index: usize) -> Self {
        Selection::Part { index }
    }

    /// Select a slide in a part
    #[must_use]
    pub fn select_slide_in_part(part_index: usize, slide_index: usize) -> Self {
        Selection::Slide {
            part_index: Some(part_index),
            slide_index,
        }
    }

    /// Select a loose slide
    #[must_use]
    pub fn select_loose_slide(slide_index: usize) -> Self {
        Selection::Slide {
            part_index: None,
            slide_index,
        }
    }
}
