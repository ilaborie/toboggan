//! UniFFI-compatible type wrappers for toboggan-core types.
//!
//! These newtypes provide FFI-safe interfaces for Swift/Kotlin. Conversion is
//! written by hand: as `From` where the shapes line up, and as named
//! constructors ([`Slide::from_core_slide`], [`PresentationState::new`]) where
//! the FFI needs something a core type does not carry on its own. [`Command`]
//! converts the other way, out to core.
//!
//! Every conversion destructures its core type **exhaustively** rather than
//! reading off the fields it happens to want. A field added to
//! [`toboggan_core`] then breaks this file, which is the one place that can
//! decide whether the phone should carry it. Read field-by-field instead, core
//! grew `notes`, `duration` and `GoTo` and the phone silently did without them
//! for months while everything still compiled.

use toboggan_client::{ConnectionStatus as CoreConnectionStatus, ErrorKind as CoreErrorKind};
use toboggan_core::{
    ClientRole as CoreClientRole, Command as CoreCommand, Slide as CoreSlide, SlideId,
    SlideKind as CoreSlideKind, State as CoreState, TalkResponse,
};

// ============================================================================
// Talk
// ============================================================================

/// A talk (presentation metadata)
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Talk {
    pub title: String,
    pub date: String,
    /// Every slide's title, in order.
    ///
    /// Titles, not slides — the deck itself comes from
    /// [`crate::TobogganClient::get_deck`]. This field was called `slides`,
    /// which invited callers to treat its length as the deck's length and index
    /// the two against each other.
    pub titles: Vec<String>,
}

impl From<TalkResponse> for Talk {
    fn from(value: TalkResponse) -> Self {
        // Destructured exhaustively; see the module docs.
        let TalkResponse {
            title,
            date,
            titles,
            footer: _,
            head: _,
            lang: _,
            // Both are per-slide and parallel to `titles`. They reach the app
            // already paired with the slide they describe, through
            // `crate::TobogganClient::get_deck`, rather than as a second list
            // for the caller to line up by index.
            step_counts: _,
            durations: _,
        } = value;

        Self {
            title,
            date: date.to_string(),
            titles,
        }
    }
}

// ============================================================================
// Slide
// ============================================================================

/// A slide kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum SlideKind {
    /// Cover slide
    Cover,
    /// Part header slide
    Part,
    /// Standard slide
    Standard,
}

/// A slide
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Slide {
    pub title: String,
    pub kind: SlideKind,
    /// How many reveals this slide has, or `None` when the server has not
    /// computed them.
    ///
    /// [`TalkResponse::step_counts`] is documented as "either absent, or exactly
    /// as long as `titles`", and **absent means not computed, not no steps**.
    /// Flattened to `0` the two were indistinguishable, and a client reading
    /// "no steps" walks past every reveal in the deck without showing one.
    ///
    /// `None` says so instead. An app that does not know should ask the server
    /// to take a step rather than a slide: `NextStep` moves to the next slide
    /// once the current one runs out, so it is right either way, while `Next`
    /// throws away any reveals that were there.
    pub step_count: Option<u32>,
    /// Speaker notes, flattened to plain text.
    ///
    /// The phone is the device a presenter actually looks at mid-talk, so this
    /// is the one field that makes it more than a clicker. Empty when the slide
    /// has none, rather than `Option`, because "no notes" and "empty notes" read
    /// the same in a UI and an optional would only push the check to every caller.
    pub notes: String,
    /// Speaking time the author planned for this slide, in whole seconds.
    ///
    /// `None` when the deck plans nothing, which is what hides the pacing
    /// readout rather than showing it as zero.
    pub duration_secs: Option<u64>,
}

impl Slide {
    /// Create a Slide from a `CoreSlide`, with the step count the server
    /// computed for it — `None` when the server has not computed any.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_core_slide(value: &CoreSlide, step_count: Option<usize>) -> Self {
        // Destructured exhaustively; see the module docs. The bindings are all
        // used or explicitly discarded, so a new core field lands here as a
        // compile error rather than as a phone quietly missing a feature.
        let CoreSlide {
            kind,
            title,
            notes,
            duration,
            style: _,
            body: _,
            terminals: _,
            body_source: _,
            hidden_in: _,
            quake_terminal_cwd: _,
            lint_disabled: _,
            source_path: _,
        } = value;

        // UniFFI requires u32, step counts are typically small
        Self {
            title: title.to_string(),
            kind: match kind {
                CoreSlideKind::Cover => SlideKind::Cover,
                CoreSlideKind::Part => SlideKind::Part,
                CoreSlideKind::Standard => SlideKind::Standard,
            },
            step_count: step_count.map(|count| count as u32),
            // `display_text` is the projection built for exactly this: anything
            // that cannot show markup. The terminal client renders notes the
            // same way.
            notes: notes.display_text().to_owned(),
            duration_secs: duration.map(|planned| planned.as_secs()),
        }
    }
}

// ============================================================================
// PresentationState
// ============================================================================

/// Presentation state.
///
/// Named `PresentationState` rather than `State` because this type crosses into
/// `SwiftUI` and Compose, and both of those have a `State` of their own. Exported
/// as `State` it shadows theirs inside the host module, so the app cannot use
/// its own framework's property wrapper without qualifying every single use.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum PresentationState {
    Init {
        total_slides: u32,
    },
    Running {
        previous: Option<u32>,
        current: u32,
        next: Option<u32>,
        current_step: u32,
        /// See [`Slide::step_count`]: `None` is "not computed", not "no steps".
        step_count: Option<u32>,
    },
    Done {
        previous: Option<u32>,
        current: u32,
        current_step: u32,
        /// See [`Slide::step_count`]: `None` is "not computed", not "no steps".
        step_count: Option<u32>,
    },
}

impl PresentationState {
    /// Create a new `PresentationState` from the core state and slides (for step counts)
    pub(crate) fn new(slides: &[Slide], value: &CoreState) -> Self {
        // No assertion that the deck is non-empty. There used to be one, because
        // the `next` calculation below computed `total_slides - 1` and would
        // underflow — but an empty deck is a state the client is legitimately in
        // before the talk has loaded, and an `assert!` across the UniFFI boundary
        // aborts the host app. The subtraction is gone instead.
        let total_slides = slides.len();

        #[allow(clippy::cast_possible_truncation)]
        // UniFFI requires u32, truncation unlikely for slide counts
        let total_slides_u32 = total_slides as u32;

        match *value {
            CoreState::Init => Self::Init {
                total_slides: total_slides_u32,
            },
            CoreState::Running {
                current,
                current_step,
            } => {
                #[allow(clippy::cast_possible_truncation)]
                // UniFFI requires u32, slide indices and step counts are typically small
                let current_index = current.index() as u32;
                let step_count = slides
                    .get(current_index as usize)
                    .and_then(|slide| slide.step_count);
                #[allow(clippy::cast_possible_truncation)]
                Self::Running {
                    previous: (current_index > 0).then(|| current_index - 1),
                    current: current_index,
                    next: ((current_index as usize + 1) < total_slides).then(|| current_index + 1),
                    current_step: current_step as u32,
                    step_count,
                }
            }
            CoreState::Done {
                current,
                current_step,
            } => {
                #[allow(clippy::cast_possible_truncation)]
                // UniFFI requires u32, slide indices and step counts are typically small
                let current_index = current.index() as u32;
                let step_count = slides
                    .get(current_index as usize)
                    .and_then(|slide| slide.step_count);
                #[allow(clippy::cast_possible_truncation)]
                Self::Done {
                    previous: (current_index > 0).then(|| current_index - 1),
                    current: current_index,
                    current_step: current_step as u32,
                    step_count,
                }
            }
        }
    }
}

// ============================================================================
// Command
// ============================================================================

/// Commands that can be sent to the server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum Command {
    // Slide navigation
    Next,
    Previous,
    First,
    Last,
    // Step navigation
    NextStep,
    PreviousStep,
    // Presentation control
    Blink,
    /// Jump straight to a slide, as the deck overview does.
    GoTo {
        slide: u32,
    },
}

impl From<Command> for CoreCommand {
    fn from(value: Command) -> Self {
        match value {
            Command::Next => Self::NextSlide,
            Command::Previous => Self::PreviousSlide,
            Command::First => Self::First,
            Command::Last => Self::Last,
            Command::NextStep => Self::NextStep,
            Command::PreviousStep => Self::PreviousStep,
            Command::Blink => Self::Blink,
            Command::GoTo { slide } => Self::GoTo {
                slide: SlideId::new(slide as usize),
            },
        }
    }
}

// ============================================================================
// ConnectionStatus
// ============================================================================

/// Connection status.
///
/// The payloads used to be dropped here, which left the app with "Reconnecting…"
/// and "Connection Error" and no way to say *which* attempt or *what* failed —
/// the two things anyone debugging a phone that will not reach the server needs.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Closed,
    Reconnecting {
        attempt: u32,
        max_attempt: u32,
        delay_secs: u64,
    },
    Error {
        message: String,
    },
}

impl From<CoreConnectionStatus> for ConnectionStatus {
    // `UniFFI` has no `usize`; a retry counter never approaches `u32`.
    #[allow(clippy::cast_possible_truncation)]
    fn from(value: CoreConnectionStatus) -> Self {
        match value {
            CoreConnectionStatus::Connecting => Self::Connecting,
            CoreConnectionStatus::Connected => Self::Connected,
            CoreConnectionStatus::Closed => Self::Closed,
            CoreConnectionStatus::Reconnecting {
                attempt,
                max_attempt,
                delay,
            } => Self::Reconnecting {
                attempt: attempt as u32,
                max_attempt: max_attempt as u32,
                delay_secs: delay.as_secs(),
            },
            CoreConnectionStatus::Error { message } => Self::Error { message },
        }
    }
}

/// The role the server granted this client.
///
/// Told, never asked for. A phone is never on the machine running the server, so
/// without a presenter token it is audience — and it has to say so rather than
/// let the user discover it by pressing a button that does nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ClientRole {
    Presenter,
    Audience,
}

impl Default for ClientRole {
    /// Audience, mirroring [`toboggan_core::ClientRole`]: a role that has not
    /// arrived is the one that can do the least. An app that guesses the other
    /// way offers controls the server will refuse.
    fn default() -> Self {
        Self::Audience
    }
}

impl From<CoreClientRole> for ClientRole {
    fn from(value: CoreClientRole) -> Self {
        match value {
            CoreClientRole::Presenter => Self::Presenter,
            CoreClientRole::Audience => Self::Audience,
        }
    }
}

/// Why an error is being reported.
///
/// The app shows a refusal inline and interrupts only for a broken connection,
/// and it needs to tell them apart *before* it reads the message. Without this
/// it matched the server's English prose for the word "watching", so rewording
/// one server string turned every permissions answer into a modal network
/// alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ErrorKind {
    /// The server answered, and its answer was a complaint.
    Server,
    /// The server could not be reached, or stopped being reachable.
    Transport,
}

impl From<CoreErrorKind> for ErrorKind {
    fn from(value: CoreErrorKind) -> Self {
        match value {
            CoreErrorKind::Server => Self::Server,
            CoreErrorKind::Transport => Self::Transport,
        }
    }
}

#[cfg(test)]
mod tests {
    use toboggan_core::{Content, SlideId, State as CoreState};

    use super::*;

    fn slide(title: &str) -> Slide {
        Slide {
            title: title.to_owned(),
            kind: SlideKind::Standard,
            step_count: Some(0),
            notes: String::new(),
            duration_secs: None,
        }
    }

    /// The deck overview jumps straight to a slide, which the FFI could not
    /// express at all: `GoTo` is in the protocol but was not in this enum.
    #[test]
    fn goto_carries_the_slide_index_across_the_boundary() {
        match CoreCommand::from(Command::GoTo { slide: 7 }) {
            CoreCommand::GoTo { slide } => assert_eq!(slide.index(), 7),
            other => panic!("expected GoTo, got {other:?}"),
        }
    }

    /// Notes are the reason to look at a phone mid-talk, and they reach it as
    /// plain text: the phone has no HTML renderer.
    #[test]
    fn notes_cross_the_boundary_as_plain_text() {
        let core = CoreSlide {
            notes: Content::html_with_alt("<em>breathe</em>", "breathe"),
            ..CoreSlide::default()
        };
        let slide = Slide::from_core_slide(&core, Some(0));
        assert_eq!(slide.notes, "breathe");
    }

    /// A slide with no notes reads as empty rather than as absent, so no caller
    /// has to unwrap to render nothing.
    #[test]
    fn a_slide_without_notes_has_empty_notes() {
        let slide = Slide::from_core_slide(&CoreSlide::default(), None);
        assert_eq!(slide.notes, "");
        assert_eq!(slide.duration_secs, None);
    }

    /// "The server has not counted the reveals" and "this slide has none" are
    /// different facts, and the phone acts differently on them. Flattened
    /// together, a deck whose step counts had not arrived played as a deck with
    /// no reveals at all.
    #[test]
    fn an_uncounted_slide_is_not_a_slide_without_steps() {
        let uncounted = Slide::from_core_slide(&CoreSlide::default(), None);
        let counted = Slide::from_core_slide(&CoreSlide::default(), Some(0));
        assert_eq!(uncounted.step_count, None);
        assert_eq!(counted.step_count, Some(0));
    }

    /// The state carries the distinction too, rather than re-flattening it one
    /// layer further out.
    #[test]
    fn an_uncounted_slide_keeps_its_unknown_step_count_in_the_state() {
        let slides = [Slide::from_core_slide(&CoreSlide::default(), None)];
        let running = CoreState::Running {
            current: SlideId::new(0),
            current_step: 0,
        };
        match PresentationState::new(&slides, &running) {
            PresentationState::Running { step_count, .. } => assert_eq!(step_count, None),
            other => panic!("expected Running, got {other:?}"),
        }
    }

    /// Every core command the phone could be asked to send has a mobile
    /// counterpart, checked in the direction that can actually rot.
    ///
    /// `From<Command> for CoreCommand` matches on the *mobile* enum, so core can
    /// grow a variant while this crate still compiles — which is exactly how
    /// `GoTo` came to be in the protocol and missing from the phone. Matching on
    /// `CoreCommand` here makes the next one a compile error instead.
    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn every_core_command_the_phone_can_send_has_a_mobile_counterpart() {
        fn mobile_equivalent(command: &CoreCommand) -> Option<Command> {
            match command {
                CoreCommand::NextSlide => Some(Command::Next),
                CoreCommand::PreviousSlide => Some(Command::Previous),
                CoreCommand::First => Some(Command::First),
                CoreCommand::Last => Some(Command::Last),
                CoreCommand::NextStep => Some(Command::NextStep),
                CoreCommand::PreviousStep => Some(Command::PreviousStep),
                CoreCommand::Blink => Some(Command::Blink),
                CoreCommand::GoTo { slide } => Some(Command::GoTo {
                    slide: slide.index() as u32,
                }),
                // Connection plumbing the app never sends by hand: the client
                // registers and pings on its own.
                CoreCommand::Register { .. }
                | CoreCommand::Unregister { .. }
                | CoreCommand::Ping => None,
            }
        }

        // Round-trips, so the pairing above is checked rather than asserted.
        for command in [
            Command::Next,
            Command::Previous,
            Command::First,
            Command::Last,
            Command::NextStep,
            Command::PreviousStep,
            Command::Blink,
            Command::GoTo { slide: 4 },
        ] {
            let core = CoreCommand::from(command);
            assert_eq!(
                mobile_equivalent(&core),
                Some(command),
                "{command:?} did not survive the round trip"
            );
        }
    }

    /// The payloads used to be dropped, leaving the app unable to say which
    /// attempt was failing or why.
    #[test]
    fn reconnecting_keeps_the_detail_the_app_needs_to_report() {
        let core = CoreConnectionStatus::Reconnecting {
            attempt: 3,
            max_attempt: 5,
            delay: std::time::Duration::from_secs(4),
        };
        match ConnectionStatus::from(core) {
            ConnectionStatus::Reconnecting {
                attempt,
                max_attempt,
                delay_secs,
            } => {
                assert_eq!((attempt, max_attempt, delay_secs), (3, 5, 4));
            }
            other => panic!("expected Reconnecting, got {other:?}"),
        }
    }

    #[test]
    fn an_error_keeps_its_message() {
        let core = CoreConnectionStatus::Error {
            message: "no route to host".to_owned(),
        };
        match ConnectionStatus::from(core) {
            ConnectionStatus::Error { message } => assert_eq!(message, "no route to host"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    /// The client is legitimately in this state before the talk has loaded.
    /// It used to hit an `assert!` that aborted the host app across the FFI
    /// boundary, guarding a `total_slides - 1` that would otherwise underflow.
    #[test]
    fn an_empty_deck_does_not_abort() {
        match PresentationState::new(&[], &CoreState::Init) {
            PresentationState::Init { total_slides } => assert_eq!(total_slides, 0),
            other => panic!("expected Init, got {other:?}"),
        }
    }

    #[test]
    fn the_last_slide_has_no_next() {
        let slides = [slide("one"), slide("two")];
        let running = CoreState::Running {
            current: SlideId::new(1),
            current_step: 0,
        };
        match PresentationState::new(&slides, &running) {
            PresentationState::Running { next, previous, .. } => {
                assert_eq!(next, None, "last slide has no next");
                assert_eq!(previous, Some(0));
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }

    #[test]
    fn a_middle_slide_has_both_neighbours() {
        let slides = [slide("one"), slide("two"), slide("three")];
        let running = CoreState::Running {
            current: SlideId::new(1),
            current_step: 0,
        };
        match PresentationState::new(&slides, &running) {
            PresentationState::Running { next, previous, .. } => {
                assert_eq!(previous, Some(0));
                assert_eq!(next, Some(2));
            }
            other => panic!("expected Running, got {other:?}"),
        }
    }
}
