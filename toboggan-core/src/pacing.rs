//! Whether the talk is running to time.
//!
//! Three questions a speaker glances down to ask — how long have I been
//! talking, should I already be further on, and what time is it — and until now
//! exactly one client could answer them. The web client's presenter chrome held
//! the only working copy, in a crate that cannot host a unit test at all: its
//! bindings `require("rioterm")`, which only a bundler resolves. This is that
//! copy, moved to where `cargo nextest` can reach it, so the terminal and
//! desktop clients can have the same answers without inventing them again.
//!
//! **Nothing here reads a clock.** [`Elapsed`] is handed the reading instead,
//! for two reasons: `std::time::Instant::now()` panics on
//! `wasm32-unknown-unknown`, and the browser is one of the callers this module
//! exists to serve; and a stopwatch you can hand a time to is a stopwatch you
//! can test.

use core::time::Duration;

/// A talk's elapsed time: a running origin plus whatever was banked before the
/// last pause.
///
/// Two fields rather than a single origin, because a paused clock has no origin
/// to measure from and a running one has no total to add up. `started` is the
/// third thing neither says — a clock paused at zero and a clock that has never
/// run look identical, and only one of them should start itself when the deck
/// is first seen running.
///
/// The `now` every method takes is any monotonically increasing duration from a
/// fixed origin: `Instant::elapsed` against a start instant on the desktop,
/// `performance.now()` in a browser. Only differences are taken, so the origin
/// does not matter as long as it does not move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Elapsed {
    /// Whether the clock has ever run; see the type's own docs.
    started: bool,
    /// When the current run began. `Some` exactly while the clock is moving.
    running_since: Option<Duration>,
    /// Everything the clock ran before the current pause.
    banked: Duration,
}

impl Elapsed {
    /// Starts the clock the first time the deck is seen running.
    ///
    /// Idempotent on purpose: opening the presenter view before the talk should
    /// not have it counting the coffee break, and the state that says "the deck
    /// is running" arrives again on every reveal.
    pub fn start_if_idle(&mut self, now: Duration) {
        if self.started {
            return;
        }
        self.started = true;
        self.running_since = Some(now);
    }

    /// Pauses a running clock, or resumes a paused one — and starts one that has
    /// never run, so the button does something before the deck has moved.
    pub fn toggle(&mut self, now: Duration) {
        self.started = true;
        match self.running_since {
            Some(since) => {
                self.banked = self.banked.saturating_add(now.saturating_sub(since));
                self.running_since = None;
            }
            None => self.running_since = Some(now),
        }
    }

    /// Back to zero, in whatever run state it was already in.
    ///
    /// A reset while paused must not silently start the talk: someone who
    /// paused to take a question and then cleared the clock has not begun
    /// speaking again.
    pub fn restart(&mut self, now: Duration) {
        self.banked = Duration::ZERO;
        if self.running_since.is_some() {
            self.running_since = Some(now);
        }
    }

    /// Whether the clock is moving.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.running_since.is_some()
    }

    /// Whether the clock has ever run, so a caller can tell a fresh clock from
    /// one deliberately paused at zero.
    #[must_use]
    pub const fn has_started(&self) -> bool {
        self.started
    }

    /// Everything on the clock: what was banked, plus the current run.
    ///
    /// Saturating, so a `now` that went backwards — a caller mixing two clocks,
    /// or one that is not as monotonic as it claims — reads as no progress
    /// rather than as a panic in front of a room.
    #[must_use]
    pub fn total(&self, now: Duration) -> Duration {
        let live = self
            .running_since
            .map_or(Duration::ZERO, |since| now.saturating_sub(since));
        self.banked.saturating_add(live)
    }

    /// Whole seconds on the clock, for the readout.
    #[must_use]
    pub fn secs(&self, now: Duration) -> u64 {
        self.total(now).as_secs()
    }
}

/// Seconds behind (positive) or ahead (negative) of the deck's own plan.
///
/// Measured against the time the *preceding* slides were meant to take, so it
/// answers "should I already have been here?" — the question a speaker glancing
/// down mid-sentence is actually asking, rather than "how much of my total have
/// I spent". `None` when the deck declares no durations at all, in which case
/// there is no plan to be late for; a deck that times only some of its slides is
/// measured against the ones it times.
///
/// `plan` is [`crate::TalkResponse::durations`], parallel to its `titles`.
///
/// ```
/// use toboggan_core::pacing::drift_secs;
///
/// let plan = [Some(60), Some(60), None];
/// // Two minutes in and still on the second slide: the first was meant to take
/// // one, so we are a minute late.
/// assert_eq!(drift_secs(&plan, 1, 120), Some(60));
/// // Two minutes in on the third: two were planned, so we are exactly on time.
/// assert_eq!(drift_secs(&plan, 2, 120), Some(0));
/// // A deck that plans nothing cannot be late.
/// assert_eq!(drift_secs(&[None, None], 1, 120), None);
/// ```
#[must_use]
pub fn drift_secs(plan: &[Option<u64>], current_index: usize, elapsed_secs: u64) -> Option<i64> {
    if plan.iter().all(Option::is_none) {
        return None;
    }
    let planned_before = plan
        .iter()
        .take(current_index)
        .filter_map(|planned| *planned)
        .sum::<u64>();
    Some(i64::try_from(elapsed_secs).ok()? - i64::try_from(planned_before).ok()?)
}

/// `mm:ss`, growing to `h:mm:ss` once a talk runs past the hour.
///
/// ```
/// use toboggan_core::pacing::format_duration;
///
/// assert_eq!(format_duration(0), "0:00");
/// assert_eq!(format_duration(95), "1:35");
/// assert_eq!(format_duration(3_725), "1:02:05");
/// ```
#[must_use]
pub fn format_duration(secs: u64) -> String {
    let (hours, minutes, seconds) = (secs / 3_600, (secs % 3_600) / 60, secs % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// The wall clock, as `HH:MM` in the local time zone.
///
/// A speaker looks at the presenter view rather than at the menu bar, and a
/// talk has a slot that ends at a wall-clock time rather than after a duration.
#[must_use]
pub fn wall_clock() -> String {
    format_clock(&jiff::Zoned::now())
}

/// `HH:MM` for a given zoned time — the half of [`wall_clock`] that can be
/// tested without waiting for the minute to turn over.
fn format_clock(now: &jiff::Zoned) -> String {
    format!("{:02}:{:02}", now.hour(), now.minute())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use core::str::FromStr as _;

    use super::*;

    /// Seconds into the talk, as the callers' monotonic reading.
    fn at(secs: u64) -> Duration {
        Duration::from_secs(secs)
    }

    #[test]
    fn a_fresh_clock_reads_zero_and_is_not_running() {
        let clock = Elapsed::default();
        assert_eq!(clock.secs(at(30)), 0);
        assert!(!clock.is_running());
        assert!(!clock.has_started());
    }

    #[test]
    fn start_if_idle_starts_once_and_only_once() {
        let mut clock = Elapsed::default();
        clock.start_if_idle(at(0));
        // The deck re-sends its state on every reveal; a second call must not
        // move the origin, or the timer would restart on every space bar.
        clock.start_if_idle(at(30));
        assert_eq!(clock.secs(at(60)), 60);
    }

    #[test]
    fn toggle_banks_the_run_and_stops_the_clock() {
        let mut clock = Elapsed::default();
        clock.start_if_idle(at(0));
        clock.toggle(at(20));
        assert!(!clock.is_running());
        assert_eq!(clock.secs(at(20)), 20);
        assert_eq!(clock.secs(at(300)), 20, "a paused clock does not advance");
    }

    #[test]
    fn toggle_resumes_from_the_bank() {
        let mut clock = Elapsed::default();
        clock.start_if_idle(at(0));
        clock.toggle(at(20));
        clock.toggle(at(100));
        assert!(clock.is_running());
        assert_eq!(clock.secs(at(110)), 30, "20 banked plus 10 since resuming");
    }

    #[test]
    fn toggle_starts_a_clock_that_has_never_run() {
        let mut clock = Elapsed::default();
        clock.toggle(at(10));
        assert!(clock.is_running());
        assert_eq!(clock.secs(at(40)), 30);
    }

    #[test]
    fn restart_while_paused_does_not_start_the_talk() {
        let mut clock = Elapsed::default();
        clock.start_if_idle(at(0));
        clock.toggle(at(20));
        clock.restart(at(30));
        assert!(!clock.is_running());
        assert_eq!(clock.secs(at(300)), 0);
    }

    #[test]
    fn restart_while_running_keeps_running_from_zero() {
        let mut clock = Elapsed::default();
        clock.start_if_idle(at(0));
        clock.restart(at(50));
        assert!(clock.is_running());
        assert_eq!(clock.secs(at(70)), 20);
    }

    #[test]
    fn a_now_that_went_backwards_reads_as_no_progress() {
        let mut clock = Elapsed::default();
        clock.start_if_idle(at(100));
        assert_eq!(clock.secs(at(40)), 0);
    }

    #[test]
    fn drift_measures_the_slides_before_this_one_not_including_it() {
        let plan = [Some(60), Some(60), Some(60)];
        // On the second slide, one minute was planned before it. Ninety seconds
        // in is thirty late — not ninety, and not thirty early, which is what
        // counting the current slide's own budget would say.
        assert_eq!(drift_secs(&plan, 1, 90), Some(30));
    }

    #[test]
    fn drift_is_negative_when_the_talk_is_ahead() {
        let plan = [Some(60), Some(60)];
        assert_eq!(drift_secs(&plan, 1, 20), Some(-40));
    }

    #[test]
    fn a_deck_with_no_durations_has_no_drift() {
        assert_eq!(drift_secs(&[None, None, None], 2, 500), None);
        assert_eq!(drift_secs(&[], 0, 500), None);
    }

    /// A slide the deck did not time budgets nothing, so time spent on it reads
    /// as lateness. That is the honest answer: the plan says the first two
    /// slides should have taken a minute between them, and they took two.
    #[test]
    fn a_partly_timed_deck_is_measured_against_what_it_times() {
        let plan = [Some(60), None, Some(60)];
        assert_eq!(drift_secs(&plan, 2, 120), Some(60));
    }

    #[test]
    fn drift_past_the_end_of_the_plan_counts_the_whole_plan() {
        let plan = [Some(60), Some(60)];
        assert_eq!(drift_secs(&plan, 99, 120), Some(0));
    }

    #[test]
    fn format_duration_grows_a_third_field_past_the_hour() {
        assert_eq!(format_duration(0), "0:00");
        assert_eq!(format_duration(9), "0:09");
        assert_eq!(format_duration(600), "10:00");
        assert_eq!(format_duration(3_599), "59:59");
        assert_eq!(format_duration(3_600), "1:00:00");
        assert_eq!(format_duration(3_725), "1:02:05");
    }

    /// Against UTC rather than a named zone: the test environment has no time
    /// zone database, and what is under test is the padding, not the offset.
    #[test]
    fn format_clock_pads_both_fields() {
        let morning = jiff::Timestamp::from_str("2026-08-27T09:05:00Z")
            .expect("a valid timestamp")
            .to_zoned(jiff::tz::TimeZone::UTC);
        assert_eq!(format_clock(&morning), "09:05");

        let evening = jiff::Timestamp::from_str("2026-08-27T16:42:00Z")
            .expect("a valid timestamp")
            .to_zoned(jiff::tz::TimeZone::UTC);
        assert_eq!(format_clock(&evening), "16:42");
    }
}
