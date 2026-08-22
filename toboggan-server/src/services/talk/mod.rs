use std::sync::Arc;

use anyhow::bail;
use toboggan_core::{Command, Content, Notification, RenderTarget, Slide, SlideId, State, Talk};
use toboggan_stats::SlideStats;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// A loaded talk together with the values derived from it.
///
/// Both fields are behind an `Arc` so a reader can take a snapshot without
/// copying the deck. `GET /api/talk` used to deep-clone every slide — rendered
/// HTML and all — on every request, and then recompute `SlideStats` for each of
/// them on top of that, which is itself several HTML parses per slide. Step
/// counts are a function of the slides, so they are computed once when a deck
/// is loaded.
///
/// `GET /api/slides` still clones: [`SlidesResponse`](toboggan_core::SlidesResponse)
/// owns its `Vec<Slide>`, so the copy is the wire type's, not this service's.
/// It is one clone rather than a clone plus a re-parse, and narrowing it means
/// changing a published response shape.
#[derive(Clone)]
struct LoadedTalk {
    /// The deck as it is presented: `hidden_in = ["web"]` slides removed.
    ///
    /// Everything that numbers slides reads this one, so a slide the deck does
    /// not show is not a slide the deck can be told to go to.
    talk: Arc<Talk>,
    /// The deck as it was authored, hiding nothing.
    ///
    /// The PDF is exported from here — it applies its own `hidden_in = ["pdf"]`
    /// filter, and a deck already stripped of its pdf-only slides would have
    /// nothing left to put in their place. The thumbnail overview reads it too,
    /// because it is an authoring view: it lists every slide and badges the ones
    /// the web will not show.
    source: Arc<Talk>,
    /// Reveal-step counts for `talk`, by index — so they line up with it.
    step_counts: Arc<[usize]>,
}

impl LoadedTalk {
    fn new(talk: Talk) -> Self {
        let presented = talk.visible_in(RenderTarget::Web).into_owned();
        let step_counts = presented
            .slides
            .iter()
            .map(|slide| SlideStats::from_slide(slide).steps)
            .collect();
        Self {
            talk: Arc::new(presented),
            source: Arc::new(talk),
            step_counts,
        }
    }

    /// How many slides the deck hides from the web.
    fn hidden_count(&self) -> usize {
        self.source.slides.len() - self.talk.slides.len()
    }
}

/// Service for managing talk content and presentation state
#[derive(Clone)]
pub struct TalkService {
    talk: Arc<RwLock<LoadedTalk>>,
    current_state: Arc<RwLock<State>>,
}

impl TalkService {
    /// Creates a new `TalkService` with the given talk
    ///
    /// # Errors
    /// Returns an error if the talk has no slides
    pub fn new(talk: Talk) -> anyhow::Result<Self> {
        if talk.slides.is_empty() {
            bail!("Empty talk, need at least one slide, got {talk:#?}");
        }
        let loaded = LoadedTalk::new(talk);
        if loaded.talk.slides.is_empty() {
            bail!(
                "Every one of the {} slides is `hidden_in = [\"web\"]`, so there is \
                 nothing to present",
                loaded.source.slides.len()
            );
        }

        info!(
            "\n=== Slides ===\n{}",
            loaded
                .talk
                .slides
                .iter()
                .enumerate()
                .map(|(index, slide)| format!("[{index:02}] {slide}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        if loaded.hidden_count() > 0 {
            info!(
                "{} slide(s) are `hidden_in = [\"web\"]` and are not served; they are \
                 still exported to PDF and listed in the slide overview",
                loaded.hidden_count()
            );
        }

        let current_state = State::default();
        let current_state = Arc::new(RwLock::new(current_state));

        Ok(Self {
            talk: Arc::new(RwLock::new(loaded)),
            current_state,
        })
    }

    /// Returns the talk title
    pub async fn title(&self) -> String {
        let talk = self.talk.read().await;
        talk.talk.title.clone()
    }

    /// Returns a shared handle to the current talk.
    pub async fn talk(&self) -> Arc<Talk> {
        Arc::clone(&self.talk.read().await.talk)
    }

    /// Returns the deck as authored, including slides hidden from the web.
    ///
    /// For the exporters, which do their own per-target filtering, and for the
    /// slide overview. Anything that presents or numbers slides wants
    /// [`Self::talk`] instead.
    pub async fn source_talk(&self) -> Arc<Talk> {
        Arc::clone(&self.talk.read().await.source)
    }

    /// Returns the per-slide reveal-step counts, computed when the deck loaded.
    pub async fn step_counts(&self) -> Arc<[usize]> {
        Arc::clone(&self.talk.read().await.step_counts)
    }

    /// Returns a clone of all slides
    pub async fn slides(&self) -> Vec<Slide> {
        let talk = self.talk.read().await;
        talk.talk.slides.clone()
    }

    /// Returns a slide by its index
    pub async fn slide_by_index(&self, slide_id: SlideId) -> Option<Slide> {
        let talk = self.talk.read().await;
        talk.talk.slides.get(slide_id.index()).cloned()
    }

    /// Returns the current presentation state
    pub async fn current_state(&self) -> State {
        let state = self.current_state.read().await;
        state.clone()
    }

    /// Handles a command and returns the notification (without broadcasting)
    pub async fn handle_command(&self, command: &Command) -> Notification {
        let mut state = self.current_state.write().await;

        match command {
            Command::Register { .. } | Command::Unregister { .. } => {
                Notification::state(state.clone())
            }
            Command::First => self.command_first(&mut state).await,
            Command::Last => self.command_last(&mut state).await,
            Command::GoTo { slide } => self.command_goto(&mut state, *slide).await,
            Command::NextSlide => self.command_next(&mut state).await,
            Command::PreviousSlide => self.command_previous(&mut state).await,
            Command::NextStep => self.command_next_step(&mut state).await,
            Command::PreviousStep => self.command_previous_step(&mut state).await,
            Command::Blink => Self::command_blink(),
            Command::Ping => Notification::PONG,
        }
    }

    /// Reloads talk and returns `TalkChange` notification (without broadcasting)
    ///
    /// # Errors
    /// Returns an error if the new talk has no slides
    pub async fn reload_talk(&self, new_talk: Talk) -> anyhow::Result<Notification> {
        if new_talk.slides.is_empty() {
            bail!("Cannot reload talk with empty slides");
        }
        // Filter before anything looks at the slides: the position below is
        // preserved by comparing the old deck against the new one, and comparing
        // a presented deck against an authored one would line the two up wrong
        // wherever a web-hidden slide sits between them.
        let loaded = LoadedTalk::new(new_talk);
        if loaded.talk.slides.is_empty() {
            bail!("Cannot reload: every slide in the new deck is `hidden_in = [\"web\"]`");
        }
        let new_talk = &loaded.talk;

        let mut state = self.current_state.write().await;
        let current_slide_id = state.current().unwrap_or(SlideId::FIRST);

        let old_talk = self.talk.read().await;
        let current_slide = old_talk.talk.slides.get(current_slide_id.index());

        // Preserve slide position: by title -> by index -> fallback to first
        let new_slide_id = Self::preserve_slide_position(
            current_slide,
            current_slide_id,
            &old_talk.talk.slides,
            &new_talk.slides,
        );

        info!(
            old_slide = current_slide_id.index(),
            new_slide = new_slide_id.index(),
            old_title = ?current_slide.map(|slide| &slide.title),
            new_title = ?new_talk.slides.get(new_slide_id.index()).map(|slide| &slide.title),
            "Talk reloaded"
        );

        // Update slide index in current state
        state.update_slide(new_slide_id);
        drop(old_talk);

        // Replace the talk
        let mut talk = self.talk.write().await;
        *talk = loaded;
        drop(talk);

        // Return TalkChange notification
        Ok(Notification::talk_change(state.clone()))
    }

    // === Private helper methods ===

    const NO_SLIDES_ERROR: &str = "No slides available";

    async fn total_slides(&self) -> usize {
        let talk = self.talk.read().await;
        talk.talk.slides.len()
    }

    /// Returns total slides count, or None if empty (for early return pattern)
    async fn require_slides(&self) -> Option<usize> {
        let total = self.total_slides().await;
        if total == 0 { None } else { Some(total) }
    }

    fn transition_to_running(state: &mut State, slide_id: SlideId) {
        *state = State::Running {
            current: slide_id,
            current_step: 0,
        };
    }

    async fn command_first(&self, state: &mut State) -> Notification {
        let Some(total_slides) = self.require_slides().await else {
            return Notification::error(Self::NO_SLIDES_ERROR.to_owned());
        };

        let should_transition = matches!(state, State::Init) || !state.is_first_slide(total_slides);

        if should_transition {
            Self::transition_to_running(state, SlideId::FIRST);
        }

        Notification::state(state.clone())
    }

    async fn command_last(&self, state: &mut State) -> Notification {
        let Some(total_slides) = self.require_slides().await else {
            return Notification::error(Self::NO_SLIDES_ERROR.to_owned());
        };

        let last_slide = SlideId::new(total_slides - 1);
        Self::navigate_to_slide(state, last_slide);
        Notification::state(state.clone())
    }

    async fn command_goto(&self, state: &mut State, slide_id: SlideId) -> Notification {
        let total_slides = self.total_slides().await;
        if slide_id.index() >= total_slides {
            return Notification::error(format!(
                "Slide index {} not found, total slides: {total_slides}",
                slide_id.index()
            ));
        }

        Self::navigate_to_slide(state, slide_id);
        Notification::state(state.clone())
    }

    async fn command_next(&self, state: &mut State) -> Notification {
        let Some(total_slides) = self.require_slides().await else {
            warn!("{}", Self::NO_SLIDES_ERROR);
            return Notification::error(Self::NO_SLIDES_ERROR.to_owned());
        };

        match state {
            State::Init => Self::transition_to_running(state, SlideId::FIRST),
            State::Running { .. } | State::Done { .. } => {
                Self::handle_next_in_running_state(state, total_slides);
            }
        }

        Notification::state(state.clone())
    }

    async fn command_previous(&self, state: &mut State) -> Notification {
        let Some(total_slides) = self.require_slides().await else {
            return Notification::error(Self::NO_SLIDES_ERROR.to_owned());
        };

        match state {
            State::Init => Self::transition_to_running(state, SlideId::FIRST),
            State::Running { .. } | State::Done { .. } => {
                if let Some(prev_slide) = state.previous(total_slides) {
                    state.update_slide(prev_slide);
                }
            }
        }

        Notification::state(state.clone())
    }

    fn command_blink() -> Notification {
        Notification::BLINK
    }

    async fn command_next_step(&self, state: &mut State) -> Notification {
        let Some(current_slide_index) = state.current() else {
            return Notification::state(state.clone());
        };

        let Some(slide) = self.slide_by_index(current_slide_index).await else {
            return Notification::state(state.clone());
        };

        let current_step = state.current_step();
        let step_count = SlideStats::from_slide(&slide).steps;
        if current_step < step_count {
            // Reveal next step within current slide
            state.update_step(current_step + 1);
        } else {
            // All steps revealed, go to first step of next slide
            let total_slides = self.total_slides().await;
            if let Some(next_slide_index) = state.next(total_slides) {
                state.update_slide(next_slide_index);
                state.update_step(0);
            }
        }

        Notification::state(state.clone())
    }

    async fn command_previous_step(&self, state: &mut State) -> Notification {
        let current_step = state.current_step();

        if current_step > 0 {
            // Just decrement step within current slide
            state.update_step(current_step - 1);
        } else {
            // At step 0, go to previous slide's last step
            let total_slides = self.total_slides().await;
            if let Some(prev_slide_index) = state.previous(total_slides)
                && let Some(prev_slide) = self.slide_by_index(prev_slide_index).await
            {
                state.update_slide(prev_slide_index);
                // Set to last step of previous slide (step_count means all steps revealed)
                let prev_step_count = SlideStats::from_slide(&prev_slide).steps;
                state.update_step(prev_step_count);
            }
        }

        Notification::state(state.clone())
    }

    fn navigate_to_slide(state: &mut State, target_slide: SlideId) {
        match state {
            State::Init => {
                Self::transition_to_running(state, target_slide);
            }
            State::Running { .. } | State::Done { .. } => {
                state.update_slide(target_slide);
            }
        }
    }

    fn handle_next_in_running_state(state: &mut State, total_slides: usize) {
        if let Some(current) = state.current() {
            if let Some(next_slide) = state.next(total_slides) {
                state.update_slide(next_slide);
            } else if state.is_last_slide(total_slides) {
                let current_step = state.current_step();
                *state = State::Done {
                    current,
                    current_step,
                };
            }
        } else {
            Self::transition_to_running(state, SlideId::FIRST);
        }
    }

    fn preserve_slide_position(
        current_slide: Option<&Slide>,
        current_id: SlideId,
        old_slides: &[Slide],
        new_slides: &[Slide],
    ) -> SlideId {
        if let Some(slide) = current_slide {
            // Try to match by title (exact match first, then case-insensitive if text)
            if let Some(position) = new_slides
                .iter()
                .position(|new_slide| new_slide.title == slide.title)
            {
                return SlideId::new(position);
            }

            // For text titles, try case-insensitive comparison
            if let Some(position) = Self::find_by_title_text(&slide.title, new_slides) {
                return SlideId::new(position);
            }
        }

        // Try to preserve index if slide count unchanged
        let current_index = current_id.index();
        if old_slides.len() == new_slides.len() && current_index < new_slides.len() {
            return current_id;
        }

        // Fallback to first slide
        SlideId::FIRST
    }

    fn find_by_title_text(title: &Content, slides: &[Slide]) -> Option<usize> {
        let title_text = match title {
            Content::Text { text } => text.to_lowercase(),
            Content::Html { alt: Some(alt), .. } => alt.to_lowercase(),
            Content::Html { raw, .. } => raw.to_lowercase(),
            Content::Empty => return None,
        };

        slides.iter().position(|slide| {
            let slide_text = match &slide.title {
                Content::Text { text } => text.to_lowercase(),
                Content::Html { alt: Some(alt), .. } => alt.to_lowercase(),
                Content::Html { raw, .. } => raw.to_lowercase(),
                Content::Empty => String::new(),
            };
            slide_text == title_text
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use toboggan_core::{RenderTarget, Slide};

    use super::*;

    /// A deck with a web-only slide, a pdf-only twin, and one shown everywhere.
    fn twinned_deck() -> Talk {
        Talk::new("Deck")
            .add_slide(Slide::new("shared"))
            .add_slide(Slide::new("live").with_hidden_in([RenderTarget::Pdf]))
            .add_slide(Slide::new("handout").with_hidden_in([RenderTarget::Web]))
            .add_slide(Slide::new("last"))
    }

    fn titles(talk: &Arc<Talk>) -> Vec<String> {
        talk.slides
            .iter()
            .map(|slide| slide.title.to_string())
            .collect()
    }

    /// The deck that is presented is the web deck: a slide the projector will
    /// never show is not a slide the presenter can land on by pressing space.
    #[tokio::test]
    async fn the_served_deck_drops_web_hidden_slides() {
        let service = TalkService::new(twinned_deck()).expect("build service");

        assert_eq!(titles(&service.talk().await), ["shared", "live", "last"]);
        assert_eq!(service.slides().await.len(), 3);
    }

    /// Numbering follows the served deck, or `GoTo 2` means one slide to the
    /// deck and another to everything that reads its index.
    #[tokio::test]
    async fn indices_and_step_counts_follow_the_served_deck() {
        let service = TalkService::new(twinned_deck()).expect("build service");

        assert_eq!(service.step_counts().await.len(), 3);
        let third = service
            .slide_by_index(SlideId::new(2))
            .await
            .expect("a third slide");
        assert_eq!(third.title.to_string(), "last");
        assert!(service.slide_by_index(SlideId::new(3)).await.is_none());
    }

    /// The exporters and the overview still get every slide: the PDF applies
    /// its own filter, and a deck already stripped of its pdf-only twins would
    /// have nothing to put in place of the live ones.
    #[tokio::test]
    async fn the_authored_deck_is_still_available_whole() {
        let service = TalkService::new(twinned_deck()).expect("build service");

        assert_eq!(
            titles(&service.source_talk().await),
            ["shared", "live", "handout", "last"]
        );
    }

    /// A reload lines the old deck up against the new one by title, and both
    /// sides have to be the *served* deck or the match slips wherever a hidden
    /// slide sits between them.
    #[tokio::test]
    async fn a_reload_keeps_the_position_it_was_on() {
        let service = TalkService::new(twinned_deck()).expect("build service");
        service
            .handle_command(&Command::GoTo {
                slide: SlideId::new(2),
            })
            .await;

        service
            .reload_talk(twinned_deck())
            .await
            .expect("reload the deck");

        let state = service.current_state().await;
        assert_eq!(state.current(), Some(SlideId::new(2)));
        assert_eq!(titles(&service.talk().await), ["shared", "live", "last"]);
    }

    /// Refused rather than served as an empty deck the clients cannot render.
    #[tokio::test]
    async fn a_deck_hidden_entirely_from_the_web_is_refused() {
        let all_hidden =
            Talk::new("Deck").add_slide(Slide::new("only").with_hidden_in([RenderTarget::Web]));

        assert!(TalkService::new(all_hidden).is_err());
    }
}
