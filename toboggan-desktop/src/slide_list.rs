//! The slide list, as a value rather than as widgets.
//!
//! Grouping, indentation and which entry is current are all decisions, and a
//! `view` returning `Element` is somewhere no test can reach:
//! iced has no snapshot harness in this workspace and an `Element` cannot be
//! inspected. So the decisions live here, where `cargo nextest` can see them,
//! and `views::sidebar` only draws what it is given.

use toboggan_core::{Slide, SlideId, SlideKind};

/// One entry in the slide list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    /// The slide this entry goes to. A part heading is a slide too, so every
    /// row has one.
    pub id: SlideId,
    /// The number printed on the slide, one-based.
    pub number: usize,
    pub label: String,
    pub kind: SlideKind,
    /// `0` for a cover or a part heading, `1` for a slide inside a part.
    pub depth: u16,
    pub is_current: bool,
}

impl Row {
    /// Whether this row heads a section.
    pub(crate) const fn is_part(&self) -> bool {
        matches!(self.kind, SlideKind::Part)
    }
}

/// The slide list, grouped under its parts.
///
/// A `SlideKind::Part` slide is a section heading *and* a slide you can stand
/// on, so it stays a row like any other — at depth zero, with the slides after
/// it indented under it, and still carrying its own id to navigate to.
pub(crate) fn rows(slides: &[Slide], current: Option<SlideId>) -> Vec<Row> {
    // A deck opens with a cover and often with slides before its first part;
    // those are not inside anything, so they stay at depth zero.
    let mut inside_a_part = false;
    slides
        .iter()
        .enumerate()
        .map(|(index, slide)| {
            let id = SlideId::new(index);
            let number = id.display_number();
            let depth = match slide.kind {
                SlideKind::Part => {
                    inside_a_part = true;
                    0
                }
                SlideKind::Cover => {
                    inside_a_part = false;
                    0
                }
                SlideKind::Standard => u16::from(inside_a_part),
            };
            Row {
                id,
                number,
                // Numbered rather than named when the deck gives no title. This
                // used to read `"{index + 1}. Slide {index}"`, printing a
                // one-based and a zero-based number in the same label.
                label: if slide.title.is_blank() {
                    format!("Slide {number}")
                } else {
                    slide.title.display_text().to_owned()
                },
                kind: slide.kind,
                depth,
                is_current: current == Some(id),
            }
        })
        .collect()
}

/// Where to snap the list so the current row is on screen.
///
/// Proportional rather than measured: every row is one button of the same
/// height, so mapping row `index` of `count` to `index / (count - 1)` pins the
/// first row to the top of the viewport, the last to the bottom, and keeps
/// everything between them inside it — without this needing to know how tall
/// the viewport is, which at `view` time it cannot. `None` when there is
/// nothing to scroll to, or nothing to scroll.
pub(crate) fn current_row_fraction(rows: &[Row]) -> Option<f32> {
    let last = rows.len().checked_sub(1)?;
    if last == 0 {
        return None;
    }
    let index = rows.iter().position(|row| row.is_current)?;
    #[allow(clippy::cast_precision_loss)]
    let fraction = index as f32 / last as f32;
    Some(fraction)
}

#[cfg(test)]
// Indexing a `Vec` a line above asserted the length of is what a test reads
// best; a panic here is a failing test, which is the point.
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use toboggan_core::Content;

    use super::*;

    fn slide(kind: SlideKind, title: &str) -> Slide {
        Slide {
            kind,
            title: if title.is_empty() {
                Content::Empty
            } else {
                Content::text(title)
            },
            ..Slide::default()
        }
    }

    /// A cover, two slides, a part, two more, a second part, one more.
    fn deck() -> Vec<Slide> {
        vec![
            slide(SlideKind::Cover, "Peut-on RIIR de tout ?"),
            slide(SlideKind::Standard, "Vous avez un message"),
            slide(SlideKind::Standard, "Nouvelle CVE"),
            slide(SlideKind::Part, "Pourquoi ?"),
            slide(SlideKind::Standard, "NIH"),
            slide(SlideKind::Standard, "Contrôle sans GC"),
            slide(SlideKind::Part, "Quoi ?"),
            slide(SlideKind::Standard, "Yak-Shaving"),
        ]
    }

    #[test]
    fn parts_group_the_slides_that_follow_them() {
        let rows = rows(&deck(), None);
        let depths = rows.iter().map(|row| row.depth).collect::<Vec<_>>();
        assert_eq!(depths, vec![0, 0, 0, 0, 1, 1, 0, 1]);
    }

    /// Slides before the first part belong to nothing, so they are not indented
    /// under a heading that is not above them.
    #[test]
    fn slides_before_the_first_part_stay_at_the_top_level() {
        let rows = rows(&deck(), None);
        assert_eq!(rows[1].depth, 0);
        assert_eq!(rows[2].depth, 0);
    }

    #[test]
    fn a_part_heading_is_still_a_row_you_can_go_to() {
        let rows = rows(&deck(), None);
        let part = rows.iter().find(|row| row.is_part()).expect("a part");
        assert_eq!(part.id, SlideId::new(3));
        assert_eq!(part.number, 4);
    }

    /// `Content::Empty` is what the server sends for an absent title, and the
    /// label used to print a one-based and a zero-based number together.
    #[test]
    fn a_slide_with_no_title_is_numbered_once_not_named() {
        let rows = rows(&[slide(SlideKind::Standard, "")], None);
        assert_eq!(rows[0].label, "Slide 1");
    }

    #[test]
    fn the_current_slide_is_marked_and_only_it() {
        let rows = rows(&deck(), Some(SlideId::new(4)));
        let marked = rows
            .iter()
            .filter(|row| row.is_current)
            .map(|row| row.number)
            .collect::<Vec<_>>();
        assert_eq!(marked, vec![5]);
    }

    #[test]
    fn current_row_fraction_pins_the_ends_of_the_list() {
        let first = rows(&deck(), Some(SlideId::new(0)));
        assert_eq!(current_row_fraction(&first), Some(0.0));

        let last = rows(&deck(), Some(SlideId::new(7)));
        assert_eq!(current_row_fraction(&last), Some(1.0));
    }

    #[test]
    fn current_row_fraction_is_none_with_nothing_to_scroll() {
        assert_eq!(current_row_fraction(&[]), None);

        // A one-slide deck has nowhere to scroll to.
        let one = rows(&[slide(SlideKind::Cover, "Only")], Some(SlideId::new(0)));
        assert_eq!(current_row_fraction(&one), None);

        // And a deck the client has not been told the position of.
        let unknown = rows(&deck(), None);
        assert_eq!(current_row_fraction(&unknown), None);
    }
}
