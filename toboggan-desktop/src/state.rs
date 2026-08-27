use std::time::Instant;

use iced::widget::markdown;
use iced::{Theme, theme};
use toboggan_client::ConnectionStatus;
use toboggan_core::pacing::Elapsed;
use toboggan_core::{ClientRole, Slide, SlideId, State as PresentationState, Talk};

/// What the presenter asked the app to look like, as opposed to what is on
/// screen right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    /// Follow the desktop's own light/dark setting, and keep following it.
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    /// Every choice, in the order the picker offers them.
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];
}

impl core::fmt::Display for ThemeChoice {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let label = match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        };
        fmt.write_str(label)
    }
}

/// Cached markdown content for a slide
#[derive(Debug, Clone, Default)]
pub(crate) struct CachedMarkdown {
    pub body_items: Vec<markdown::Item>,
    pub notes_items: Vec<markdown::Item>,
}

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    pub connection_status: ConnectionStatus,
    pub talk: Option<Talk>,
    pub slides: Vec<Slide>,
    /// Step counts per slide (from server's `TalkResponse`).
    pub step_counts: Vec<usize>,
    /// Planned seconds per slide, from `TalkResponse::durations`.
    ///
    /// Empty, or all `None`, when the deck plans nothing — which is what hides
    /// the pacing readout rather than having it invent a schedule the speaker
    /// never set.
    pub durations: Vec<Option<u64>>,
    pub cached_markdown: Vec<CachedMarkdown>,
    pub presentation_state: Option<PresentationState>,
    pub current_slide: Option<SlideId>,
    pub show_help: bool,
    pub show_sidebar: bool,
    pub fullscreen: bool,
    /// The role the server granted, once it has said.
    ///
    /// `None` before the handshake answers — not the same as "audience", so the
    /// footer says nothing rather than guessing. A client connecting across the
    /// network without a token can watch but not navigate, and used to learn
    /// that by pressing a key and being refused.
    pub role: Option<ClientRole>,
    pub error_message: Option<String>,

    /// The talk's clock, and the monotonic origin it is measured against.
    ///
    /// `Instant` stays in this struct: `pacing::Elapsed` is handed
    /// `timer_origin.elapsed()` so its arithmetic stays testable, and off a
    /// clock that panics in a browser — the web presenter view is the other
    /// caller that module exists for.
    pub timer_origin: Instant,
    pub elapsed: Elapsed,

    /// The slide number being typed, digit by digit. See `toboggan_core::goto`.
    pub goto_target: Option<usize>,

    /// What the presenter asked for.
    pub theme_choice: ThemeChoice,
    /// The last colour scheme the desktop reported.
    ///
    /// Kept here because `App::view` is handed the state and never the theme, so
    /// this is the only place that can resolve [`ThemeChoice::System`] into a
    /// concrete `Theme` for the style closures that need a palette.
    pub system_mode: theme::Mode,
}

/// Parse slides into cached markdown items
pub(crate) fn parse_slides_markdown(slides: &[Slide]) -> Vec<CachedMarkdown> {
    slides
        .iter()
        .map(|slide| {
            let body_text = slide.body.display_text().to_owned();
            let notes_text = slide.notes.display_text().to_owned();

            CachedMarkdown {
                body_items: markdown::parse(&body_text).collect(),
                notes_items: markdown::parse(&notes_text).collect(),
            }
        })
        .collect()
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connection_status: ConnectionStatus::Closed,
            talk: None,
            slides: Vec::new(),
            step_counts: Vec::new(),
            durations: Vec::new(),
            cached_markdown: Vec::new(),
            presentation_state: None,
            current_slide: None,
            show_help: false,
            show_sidebar: true,
            fullscreen: false,
            role: None,
            error_message: None,
            goto_target: None,
            timer_origin: Instant::now(),
            elapsed: Elapsed::default(),
            theme_choice: ThemeChoice::default(),
            system_mode: theme::Mode::None,
        }
    }
}

impl AppState {
    /// The theme actually on screen.
    ///
    /// The single source of truth for both `App::theme` and every view that
    /// needs a palette. Three views used to reach for `Theme::Dark` directly,
    /// which is what made a light theme unreachable however `App::theme` was
    /// changed.
    pub(crate) fn theme(&self) -> Theme {
        match self.theme_choice {
            ThemeChoice::System => <Theme as theme::Base>::default(self.system_mode),
            ThemeChoice::Light => Theme::Light,
            ThemeChoice::Dark => Theme::Dark,
        }
    }

    /// The reading the clock is measured against.
    pub(crate) fn now(&self) -> std::time::Duration {
        self.timer_origin.elapsed()
    }

    /// Seconds on the elapsed clock.
    pub(crate) fn elapsed_secs(&self) -> u64 {
        self.elapsed.secs(self.now())
    }

    /// How far behind (positive) or ahead (negative) of the deck's plan the
    /// talk is running, or `None` when the deck declares no durations.
    pub(crate) fn drift_secs(&self) -> Option<i64> {
        let index = self.current_slide?.index();
        toboggan_core::pacing::drift_secs(&self.durations, index, self.elapsed_secs())
    }

    pub(crate) fn current_slide(&self) -> Option<&Slide> {
        self.current_slide
            .and_then(|id| self.slides.get(id.index()))
    }

    pub(crate) fn current_markdown(&self) -> Option<&CachedMarkdown> {
        self.current_slide
            .and_then(|id| self.cached_markdown.get(id.index()))
    }

    pub(crate) fn next_slide(&self) -> Option<&Slide> {
        if let Some(current_id) = self.current_slide {
            let next_idx = current_id.index() + 1;
            self.slides.get(next_idx)
        } else {
            None
        }
    }

    pub(crate) fn slide_index(&self) -> Option<(usize, usize)> {
        self.current_slide
            .map(|id| (id.display_number(), self.slides.len()))
    }

    /// Returns `(current_step, step_count)` for the current slide.
    #[must_use]
    pub(crate) fn step_info(&self) -> Option<(usize, usize)> {
        let slide_id = self.current_slide?;
        let step_count = self.step_counts.get(slide_id.index()).copied().unwrap_or(0);
        self.presentation_state
            .as_ref()
            .map(|state| state.step_info(step_count))
    }
}
