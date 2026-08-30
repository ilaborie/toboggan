use std::sync::Arc;

use anyhow::bail;
use toboggan_core::{
    Command, Content, Notification, RenderTarget, Slide, SlideId, SlideKind, SlideOutline, State,
    Talk,
};
use toboggan_stats::{SlideStats, content_plain_text, notes_plain_text, slide_plain_text};
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
    /// Where each presented slide sits in `source`, by presented index.
    ///
    /// The two lists are numbered differently on purpose, and the difference is
    /// a trap: everything that presents counts over `talk`, while the slide
    /// overview — an authoring view, which lists every slide and badges the
    /// hidden ones — counts over `source`. Anything holding both a presented
    /// index and an authored artefact needs this to cross between them, and the
    /// slide picker is exactly that: `Command::GoTo` speaks the first index
    /// space and `thumb-NNNN.png` is named in the second.
    source_indexes: Arc<[usize]>,
    /// Every presented slide as plain text, for the slide picker.
    ///
    /// Numbered over `talk`, like `step_counts`, so a picker cell's index is
    /// one `Command::GoTo` takes. Built at load for the same reason: searching
    /// a deck means parsing every slide's HTML twice — body and notes — which
    /// is not something to do per request.
    outline: Arc<[SlideOutline]>,
}

impl LoadedTalk {
    fn new(talk: Talk) -> Self {
        let presented = talk.visible_in(RenderTarget::Web).into_owned();
        let step_counts = presented
            .slides
            .iter()
            .map(|slide| SlideStats::from_slide(slide).steps)
            .collect();
        // The same predicate `visible_in` filters on, walked over the authored
        // list so the positions that survive are recorded in order. Derived here
        // rather than recomputed per request, for the reason `step_counts` is:
        // a number that must line up with `talk` is safest computed beside it.
        let source_indexes = talk
            .slides
            .iter()
            .enumerate()
            .filter(|(_, slide)| !slide.hidden_in.contains(&RenderTarget::Web))
            .map(|(index, _)| index)
            .collect();
        let outline = build_outline(&presented);
        Self {
            talk: Arc::new(presented),
            source: Arc::new(talk),
            step_counts,
            source_indexes,
            outline,
        }
    }

    /// How many slides the deck hides from the web.
    fn hidden_count(&self) -> usize {
        self.source.slides.len() - self.talk.slides.len()
    }
}

/// The searchable outline of a deck, in presented order.
///
/// A part slide carries no `part` of its own — it *is* the divider — and every
/// slide after it carries its title until the next one, which is the same walk
/// the static overview does when it groups its cards.
fn build_outline(presented: &Talk) -> Arc<[SlideOutline]> {
    let mut current_part: Option<String> = None;
    presented
        .slides
        .iter()
        .map(|slide| {
            let title = content_plain_text(&slide.title).unwrap_or_default();
            // A divider opens the part it names and is not a member of it, so
            // both facts fall out of one match: it takes the title as the part
            // for everything after it, and carries `None` itself.
            let part = match slide.kind {
                SlideKind::Part => {
                    current_part = Some(title.clone());
                    None
                }
                _ => current_part.clone(),
            };
            SlideOutline {
                part,
                title,
                text: slide_plain_text(slide),
                notes: notes_plain_text(slide),
            }
        })
        .collect()
}

/// Where the deck is, and the number of the change that put it there.
///
/// One lock over both halves, so nothing can pair a state with a number that
/// belongs to a different change — which would defeat the whole purpose of
/// having one. Every mutation of either goes through [`TalkService`].
#[derive(Debug, Default)]
struct Timeline {
    state: State,
    seq: u64,
}

impl Timeline {
    /// Records a change and hands back its number.
    ///
    /// Counts from one, so the first real change stays distinguishable from
    /// [`Notification::UNNUMBERED`].
    const fn advance(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }
}

/// Service for managing talk content and presentation state
#[derive(Clone)]
pub struct TalkService {
    talk: Arc<RwLock<LoadedTalk>>,
    current_state: Arc<RwLock<Timeline>>,
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

        let current_state = Arc::new(RwLock::new(Timeline::default()));

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

    /// Returns the deck as a searchable outline, computed when it loaded.
    ///
    /// Numbered over the presented deck, so an entry's `index` is one
    /// `Command::GoTo` and `/overview/slide/{index}` both take.
    pub async fn outline(&self) -> Arc<[SlideOutline]> {
        Arc::clone(&self.talk.read().await.outline)
    }

    /// Returns the per-slide reveal-step counts, computed when the deck loaded.
    pub async fn step_counts(&self) -> Arc<[usize]> {
        Arc::clone(&self.talk.read().await.step_counts)
    }

    /// Where a presented slide sits in the deck as authored.
    ///
    /// `None` when `presented` is past the end.
    ///
    /// The two numbers differ because a `hidden_in = ["web"]` slide is dropped
    /// from what is presented and kept in what was authored — so everything that
    /// presents counts over one list, and the slide overview, which shows every
    /// slide and badges the hidden ones, is named over the other. The presenter
    /// view holds both at once and needs this to cross between them: its slide
    /// picker and its next-slide pane are both numbered over the presented deck
    /// and both drawn from `thumb-NNNN.png`, which is not.
    pub async fn source_index(&self, presented: usize) -> Option<usize> {
        self.talk
            .read()
            .await
            .source_indexes
            .get(presented)
            .copied()
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
        let timeline = self.current_state.read().await;
        timeline.state.clone()
    }

    /// The current state as a notification, numbered where it stands.
    ///
    /// What a client is told the moment it connects. It carries the number so
    /// that the client has a baseline to compare later frames against; without
    /// one it would have to treat the first change as unordered.
    pub async fn current_notification(&self) -> Notification {
        let timeline = self.current_state.read().await;
        Notification::state(timeline.state.clone()).numbered(timeline.seq)
    }

    /// Handles a command and returns the notification (without broadcasting)
    pub async fn handle_command(&self, command: &Command) -> Notification {
        let mut timeline = self.current_state.write().await;
        let state = &mut timeline.state;

        let notification = match command {
            Command::Register { .. } | Command::Unregister { .. } => {
                Notification::state(state.clone())
            }
            Command::First => self.command_first(state).await,
            Command::Last => self.command_last(state).await,
            Command::GoTo { slide } => self.command_goto(state, *slide).await,
            Command::NextSlide => self.command_next(state).await,
            Command::PreviousSlide => self.command_previous(state).await,
            Command::NextStep => self.command_next_step(state).await,
            Command::PreviousStep => self.command_previous_step(state).await,
            Command::Blink => Self::command_blink(),
            Command::Ping => Notification::PONG,
        };

        // Numbered here rather than inside each helper: this is the one place
        // that holds the write lock, so it is the only place that can promise
        // the number and the state it labels came out of the same change.
        //
        // Only a notification that carries state gets one. A helper that
        // refused — an out-of-range slide, an empty deck — returns an `Error`,
        // and `Blink` and `Ping` move nothing; advancing for those would burn
        // numbers on changes that never happened.
        //
        // `Register`/`Unregister` *do* advance, even though they leave the deck
        // where it was: they answer with a `State`, and a fresh number against
        // an unchanged value costs a client nothing — it applies what it
        // already had — while the exception would have to be remembered here
        // and mirrored in every client.
        if matches!(
            notification,
            Notification::State { .. } | Notification::TalkChange { .. }
        ) {
            let seq = timeline.advance();
            return notification.numbered(seq);
        }

        notification
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

        let mut timeline = self.current_state.write().await;
        let current_slide_id = timeline.state.current().unwrap_or(SlideId::FIRST);

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
        timeline.state.update_slide(new_slide_id);
        drop(old_talk);

        // Replace the talk
        let mut talk = self.talk.write().await;
        *talk = loaded;
        drop(talk);

        // Return TalkChange notification, on the same counter as every other
        // change: a reload moves the deck under the client, so a client must be
        // able to order it against the navigation either side of it.
        let seq = timeline.advance();
        Ok(Notification::talk_change(timeline.state.clone()).numbered(seq))
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

    /// The picker's cells are `Command::GoTo` targets, so its list has to be
    /// the presented one — the handout slide is absent, and `last` is entry 2
    /// rather than entry 3.
    #[tokio::test]
    async fn the_outline_is_numbered_over_the_presented_deck() {
        let service = TalkService::new(twinned_deck()).expect("build service");
        let outline = service.outline().await;

        let titles = outline
            .iter()
            .map(|slide| slide.title.as_str())
            .collect::<Vec<_>>();
        assert_eq!(titles, ["shared", "live", "last"]);
        // The outline is read by position, and `slides()` is the list the deck
        // can be told to go to: a picker cell drawn from entry N sends
        // `GoTo(N)`, so the two lists have to be the same length.
        assert_eq!(outline.len(), service.slides().await.len());
    }

    /// The one place the two index spaces meet, and the one that fails silently
    /// when it is wrong: the thumbnails on disk are named over the deck as
    /// *authored*, so crossing to them from a presented index has to skip the
    /// slides the web never sees.
    ///
    /// In `twinned_deck` the handout is authored slide 2, so presented `last`
    /// is authored 3 — one past where a naive identity mapping would look, and
    /// the picture it would show is the *handout's*, under `last`'s number.
    #[tokio::test]
    async fn a_presented_index_crosses_to_its_authored_slide() {
        let service = TalkService::new(twinned_deck()).expect("build service");

        assert_eq!(service.source_index(0).await, Some(0)); // shared
        assert_eq!(service.source_index(1).await, Some(1)); // live
        assert_eq!(service.source_index(2).await, Some(3)); // last, skipping handout
        // Past the end of the presented deck: no picture, rather than someone
        // else's.
        assert_eq!(service.source_index(3).await, None);
    }

    /// What the speaker actually searches for: the words on the slide and the
    /// words they meant to say about it, both as plain text rather than markup.
    ///
    /// Two parts, not one: a slide carries its part until the *next* divider,
    /// and a `current_part` that were assigned once and never reassigned would
    /// pass a deck that has only one.
    #[tokio::test]
    async fn the_outline_carries_body_text_notes_and_parts() {
        let talk = Talk::new("Deck")
            .add_slide(Slide {
                kind: SlideKind::Part,
                title: Content::text("Le début"),
                ..Default::default()
            })
            .add_slide(Slide {
                title: Content::text("Ownership"),
                body: Content::html("<p>Le <b>borrow checker</b></p>"),
                notes: Content::html("<p>Insister sur la durée de vie</p>"),
                ..Default::default()
            })
            .add_slide(Slide {
                kind: SlideKind::Part,
                title: Content::text("La suite"),
                ..Default::default()
            })
            .add_slide(Slide {
                title: Content::text("Lifetimes"),
                ..Default::default()
            });
        let service = TalkService::new(talk).expect("build service");
        let outline = service.outline().await;

        let divider = outline.first().expect("a part slide");
        let slide = outline.get(1).expect("a slide after it");
        // The divider is not a member of its own part.
        assert_eq!(divider.part, None);
        assert_eq!(slide.part.as_deref(), Some("Le début"));
        assert_eq!(slide.text, "Le borrow checker");
        assert_eq!(slide.notes, "Insister sur la durée de vie");

        // The second divider takes over from the first.
        assert_eq!(outline.get(2).expect("a second part slide").part, None);
        assert_eq!(
            outline
                .get(3)
                .expect("a slide in the second part")
                .part
                .as_deref(),
            Some("La suite")
        );
    }

    /// A mermaid fence renders to `<svg>`, and its labels are words on the
    /// slide: a speaker who remembers the diagram has to be able to find it.
    #[tokio::test]
    async fn the_outline_searches_inside_diagrams_and_figures() {
        let talk = Talk::new("Deck").add_slide(Slide {
            title: Content::text("Architecture"),
            body: Content::html(
                "<svg><text>retry loop</text></svg><figure><figcaption>Le schéma</figcaption></figure>",
            ),
            ..Default::default()
        });
        let service = TalkService::new(talk).expect("build service");
        let outline = service.outline().await;

        let slide = outline.first().expect("a slide");
        assert!(slide.text.contains("retry loop"), "got {:?}", slide.text);
        assert!(slide.text.contains("Le schéma"), "got {:?}", slide.text);
    }

    /// Refused rather than served as an empty deck the clients cannot render.
    #[tokio::test]
    async fn a_deck_hidden_entirely_from_the_web_is_refused() {
        let all_hidden =
            Talk::new("Deck").add_slide(Slide::new("only").with_hidden_in([RenderTarget::Web]));

        assert!(TalkService::new(all_hidden).is_err());
    }
}
