//! Typing a slide number, one digit at a time.
//!
//! The presenter types the number printed on the slide and presses `Enter`.
//! Every client that offers this had its own copy of the arithmetic — the web
//! client inlined it in an event closure, the TUI in a private method — and
//! both copies had the same two defects: a leading `0` was accepted and then
//! jumped to slide 1, and neither could be tested where it lived. The web
//! client's copy still cannot be: that crate's bindings `require("rioterm")`,
//! so it will not load outside a bundler.
//!
//! This is the one copy, in the crate that owns [`SlideId`] and [`Command`].

use crate::{Command, SlideId};

/// The largest slide number that can be typed.
///
/// Four digits is more slides than a talk has ever had, and the cap is what
/// stops a leaned-on key from overflowing the running multiplication.
pub const MAX_GOTO_TARGET: usize = 9_999;

/// Appends `digit` to the number typed so far, or refuses it.
///
/// `None` means the keystroke changes nothing and the pending number — if any —
/// survives: past [`MAX_GOTO_TARGET`] the result could not land on a slide, and
/// a leading zero is not the start of a slide number, because slides are
/// numbered from one.
///
/// ```
/// # use toboggan_core::accumulate_goto;
/// assert_eq!(accumulate_goto(None, 1), Some(1));
/// assert_eq!(accumulate_goto(Some(1), 2), Some(12));
/// // A leading zero starts nothing; a zero inside a number is a digit.
/// assert_eq!(accumulate_goto(None, 0), None);
/// assert_eq!(accumulate_goto(Some(1), 0), Some(10));
/// ```
#[must_use]
pub fn accumulate_goto(pending: Option<usize>, digit: u8) -> Option<usize> {
    let digit = usize::from(digit);
    if pending.is_none() && digit == 0 {
        return None;
    }
    let typed = pending.unwrap_or(0) * 10 + digit;
    (typed <= MAX_GOTO_TARGET).then_some(typed)
}

/// The command for the slide number a presenter typed.
///
/// They type what is printed on the slide, which is one-based; [`SlideId`] is
/// an index. This is the only place that conversion happens, so an off-by-one
/// here would misroute every jump in a talk.
///
/// ```
/// # use toboggan_core::{goto_command, Command, SlideId};
/// assert_eq!(goto_command(1), Command::GoTo { slide: SlideId::new(0) });
/// assert_eq!(goto_command(12), Command::GoTo { slide: SlideId::new(11) });
/// ```
#[must_use]
pub fn goto_command(number: usize) -> Command {
    Command::GoTo {
        slide: SlideId::new(number.saturating_sub(1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_accumulate_left_to_right() {
        assert_eq!(accumulate_goto(None, 1), Some(1));
        assert_eq!(accumulate_goto(Some(1), 2), Some(12));
        assert_eq!(accumulate_goto(Some(12), 3), Some(123));
        assert_eq!(accumulate_goto(Some(123), 4), Some(1234));
    }

    /// The digit is refused and the number typed so far survives, so a fifth
    /// keystroke does not cost the presenter the four they meant.
    #[test]
    fn a_digit_past_the_cap_is_refused() {
        assert_eq!(accumulate_goto(Some(1234), 5), None);
        assert_eq!(accumulate_goto(Some(MAX_GOTO_TARGET), 0), None);
    }

    /// Both clients accepted a leading zero, showed `→ 0`, and then jumped to
    /// slide 1 through `saturating_sub`.
    #[test]
    fn a_leading_zero_is_not_a_slide_number() {
        assert_eq!(accumulate_goto(None, 0), None);
        assert_eq!(accumulate_goto(Some(1), 0), Some(10));
        assert_eq!(accumulate_goto(Some(10), 0), Some(100));
    }

    #[test]
    fn the_typed_number_is_one_based() {
        assert_eq!(
            goto_command(1),
            Command::GoTo {
                slide: SlideId::FIRST
            }
        );
        assert_eq!(
            goto_command(42),
            Command::GoTo {
                slide: SlideId::new(41)
            }
        );
    }

    /// Nothing should be able to type `0`, but `saturating_sub` is what keeps a
    /// stray one from wrapping to the last slide.
    #[test]
    fn slide_zero_does_not_wrap() {
        assert_eq!(
            goto_command(0),
            Command::GoTo {
                slide: SlideId::FIRST
            }
        );
    }
}
