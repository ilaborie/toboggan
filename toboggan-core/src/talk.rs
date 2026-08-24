use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{Date, RenderTarget, Slide};

/// A whole deck: what it is called, when it is given, and its slides in order.
///
/// Built by the parser from a folder of Markdown, or deserialized from a
/// `.toml` that `toboggan build` produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Talk {
    /// The deck's title, from `_cover.md` or from configuration.
    pub title: String,
    /// The date the talk is given.
    pub date: Date,
    /// Markup shown at the foot of every slide, from `_footer.html`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,
    /// Markup injected into `<head>`, from `_head.html` — fonts, custom CSS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    /// Typst preamble replacing the generated one, from `_preamble.typ`.
    ///
    /// The `_head.html` of the PDF side, except that it *replaces* rather than
    /// appends: the generated preamble picks the touying theme, the aspect ratio
    /// and the base text size, and none of those can be taken back by anything
    /// written after them. A deck that sets this owns every import the rendered
    /// body needs — touying (`#slide`, `#title-slide`, `#new-section-slide`),
    /// codly and codly-languages for code fences, gentle-clues for GitHub
    /// alerts, mitex for `$…$` math.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typst_preamble: Option<String>,
    /// BCP 47 language tag for the deck, e.g. `fr` or `pt-BR`.
    ///
    /// Becomes the `lang` attribute on every page Toboggan renders. It is what
    /// tells a screen reader how to pronounce the deck and a browser which
    /// hyphenation and quotation rules to apply, so a French talk announced as
    /// English is read aloud as gibberish. `None` leaves the default, `en`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Default working directory for the `QuakeTerminal` overlay, used when a slide
    /// does not declare its own. Relative paths resolve against [`Talk::source_dir`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_terminal_cwd: Option<String>,
    /// Directory of the talk's source file(s). Populated by the loader; not serialized.
    #[serde(default, skip_serializing, skip_deserializing)]
    pub source_dir: Option<String>,
    /// Every slide, in the order they are presented.
    pub slides: Vec<Slide>,
}

impl Talk {
    /// The deck as `target` sees it, with the slides that named `target` in
    /// their `hidden_in` removed.
    ///
    /// Borrowed when the deck hides nothing, which is the usual case, so asking
    /// costs no clone. The slides that survive keep their order; their indices
    /// do not survive, which is why this is applied once at the edge that owns
    /// the numbering rather than per-request.
    #[must_use]
    pub fn visible_in(&self, target: RenderTarget) -> Cow<'_, Self> {
        if self.slides.iter().any(|slide| slide.is_hidden_from(target)) {
            let mut visible = self.clone();
            visible.slides.retain(|slide| !slide.is_hidden_from(target));
            Cow::Owned(visible)
        } else {
            Cow::Borrowed(self)
        }
    }

    /// An empty deck with the given title, dated today.
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        let date = Date::today();
        let slides = Vec::new();
        let footer = None;
        let head = None;

        Self {
            title,
            date,
            footer,
            head,
            typst_preamble: None,
            lang: None,
            default_terminal_cwd: None,
            source_dir: None,
            slides,
        }
    }

    /// The deck's language tag, falling back to `en`.
    #[must_use]
    pub fn lang(&self) -> &str {
        self.lang.as_deref().unwrap_or("en")
    }

    #[must_use]
    /// Sets the date the talk is given.
    pub fn with_date(mut self, date: Date) -> Self {
        self.date = date;
        self
    }

    #[must_use]
    /// Sets the footer markup shown on every slide.
    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    #[must_use]
    /// Sets markup to inject into the document `<head>`.
    pub fn with_head(mut self, head: impl Into<String>) -> Self {
        self.head = Some(head.into());
        self
    }

    #[must_use]
    /// Sets the deck's BCP 47 language tag.
    pub fn with_lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = Some(lang.into());
        self
    }

    #[must_use]
    /// Sets the fallback working directory for the quake terminal.
    pub fn with_default_terminal_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.default_terminal_cwd = Some(cwd.into());
        self
    }

    #[must_use]
    /// Records the directory the deck was loaded from.
    pub fn with_source_dir(mut self, dir: impl Into<String>) -> Self {
        self.source_dir = Some(dir.into());
        self
    }

    #[must_use]
    /// Appends a slide.
    pub fn add_slide(mut self, slide: Slide) -> Self {
        self.slides.push(slide);
        self
    }
}

/// The body of `GET /api/talk`: everything about a deck except its slides.
///
/// Carries slide *titles* rather than slides, so a client can render an outline,
/// a progress bar and a presenter view without pulling the whole deck — and so
/// the server can answer from a shared talk instead of deep-cloning every
/// slide's rendered HTML on every request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TalkResponse {
    /// The deck's title.
    pub title: String,
    /// The date the talk is given.
    pub date: Date,
    /// Footer markup, if the deck has any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,
    /// `<head>` markup, if the deck has any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    /// BCP 47 language tag; see [`Talk::lang`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Every slide's title, in order.
    pub titles: Vec<String>,
    /// Step counts per slide, for clients that show step progress. Either
    /// absent, or exactly as long as `titles` and read against it by index.
    // Absent means "not computed", not "no steps". The conversions below cannot
    // fill it: counting a slide's reveals means parsing its HTML, which lives
    // in `toboggan-stats`, and this crate depends on nothing in the workspace.
    // So the response is built here and completed by the server through
    // `with_step_counts`, which is what keeps the two in step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub step_counts: Vec<usize>,
    /// Planned speaking time per slide, in seconds, from each slide's
    /// `duration` front matter. `None` where the author did not say.
    ///
    /// Parallel to `titles`, and here rather than fetched per slide because the
    /// presenter view needs the *whole* plan to work out whether the talk is
    /// running early or late — one number per slide is far less than the deck.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub durations: Vec<Option<u64>>,
}

impl Default for TalkResponse {
    fn default() -> Self {
        Self {
            title: String::default(),
            date: Date::today(),
            footer: None,
            head: None,
            lang: None,
            titles: vec![],
            step_counts: vec![],
            durations: vec![],
        }
    }
}

impl From<&Talk> for TalkResponse {
    /// Builds a response without consuming the talk.
    ///
    /// `TalkResponse` carries titles, not slides, so the server can answer
    /// `GET /api/talk` from a shared talk instead of deep-cloning every slide's
    /// rendered HTML per request just to throw it away.
    fn from(value: &Talk) -> Self {
        Self {
            title: value.title.clone(),
            date: value.date,
            footer: value.footer.clone(),
            head: value.head.clone(),
            lang: value.lang.clone(),
            titles: value.slides.iter().map(|it| it.title.to_string()).collect(),
            step_counts: vec![], // Populated by server with SlideStats
            durations: value.slides.iter().map(planned_seconds).collect(),
        }
    }
}

impl TalkResponse {
    /// Attaches step counts, which only the server can compute.
    ///
    /// Rejects a count that does not line up with `titles` rather than
    /// storing it: the two are read by index — `step_counts[i]` is the number
    /// of reveals on the slide `titles[i]` names — so a length mismatch puts
    /// one slide's progress against another slide's name, silently and only on
    /// the slides past the point where they diverge.
    #[must_use]
    pub fn with_step_counts(mut self, step_counts: Vec<usize>) -> Self {
        debug_assert_eq!(
            step_counts.len(),
            self.titles.len(),
            "step counts must line up with the slides they describe"
        );
        if step_counts.len() == self.titles.len() {
            self.step_counts = step_counts;
        }
        self
    }
}

/// A slide's planned speaking time in whole seconds.
fn planned_seconds(slide: &Slide) -> Option<u64> {
    slide.duration.map(|duration| duration.as_secs())
}

impl From<Talk> for TalkResponse {
    fn from(value: Talk) -> Self {
        let Talk {
            title,
            date,
            footer,
            head,
            // A Typst preamble is for the PDF renderer; no client has a use for it.
            typst_preamble: _,
            lang,
            default_terminal_cwd: _,
            source_dir: _,
            slides,
        } = value;
        let titles = slides.iter().map(|it| it.title.to_string()).collect();
        let durations = slides.iter().map(planned_seconds).collect();

        Self {
            title,
            date,
            footer,
            head,
            lang,
            titles,
            step_counts: vec![], // Populated by server with SlideStats
            durations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Content, Slide};

    /// The common deck hides nothing, and asking what the web sees must not
    /// cost it a clone of every slide.
    #[test]
    fn a_deck_that_hides_nothing_is_borrowed() {
        let talk = Talk::new("t")
            .add_slide(Slide::new("one"))
            .add_slide(Slide::new("two"));

        let visible = talk.visible_in(RenderTarget::Web);
        assert!(matches!(visible, Cow::Borrowed(_)));
        assert_eq!(visible.slides.len(), 2);
    }

    /// Each target sees only what was not hidden from it — and the two targets
    /// disagree, which is the whole point of the twin-slide pattern.
    #[test]
    fn each_target_sees_only_its_own_slides() {
        let talk = Talk::new("t")
            .add_slide(Slide::new("shared"))
            .add_slide(Slide::new("live").with_hidden_in([RenderTarget::Pdf]))
            .add_slide(Slide::new("handout").with_hidden_in([RenderTarget::Web]));

        let web = talk.visible_in(RenderTarget::Web);
        assert_eq!(
            web.slides
                .iter()
                .map(|slide| slide.title.to_string())
                .collect::<Vec<_>>(),
            ["shared", "live"]
        );

        let pdf = talk.visible_in(RenderTarget::Pdf);
        assert_eq!(
            pdf.slides
                .iter()
                .map(|slide| slide.title.to_string())
                .collect::<Vec<_>>(),
            ["shared", "handout"]
        );
    }

    /// A slide can be hidden from everything; the deck is then empty for both,
    /// which callers have to handle rather than being handed a phantom slide.
    #[test]
    fn a_slide_hidden_from_every_target_survives_nowhere() {
        let talk = Talk::new("t").add_slide(
            Slide::new("nowhere").with_hidden_in([RenderTarget::Web, RenderTarget::Pdf]),
        );

        assert!(talk.visible_in(RenderTarget::Web).slides.is_empty());
        assert!(talk.visible_in(RenderTarget::Pdf).slides.is_empty());
    }

    /// An untitled slide contributes an empty name, not a placeholder that
    /// every client then renders as if it were the author's own title.
    #[test]
    fn untitled_slides_do_not_get_a_placeholder_title() {
        let mut talk = Talk::new("Deck");
        talk.slides = vec![Slide {
            title: Content::Empty,
            ..Default::default()
        }];

        let response = TalkResponse::from(talk);
        assert_eq!(response.titles, vec![String::new()]);
    }

    /// Step counts and titles are read against each other by index, so a set
    /// that does not line up would put one slide's progress under another
    /// slide's name — on every slide past the point they diverge.
    #[test]
    fn step_counts_that_do_not_line_up_are_refused() {
        let talk = Talk::new("t")
            .add_slide(Slide::new("one"))
            .add_slide(Slide::new("two"));
        let response = TalkResponse::from(&talk);
        assert_eq!(response.titles.len(), 2);

        let filled = response.with_step_counts(vec![3, 1]);
        assert_eq!(filled.step_counts, vec![3, 1]);
    }

    /// Until the server attaches them, there are none — which is "not computed"
    /// rather than "no steps", and is why the field is allowed to be empty.
    #[test]
    fn a_converted_talk_carries_no_step_counts_yet() {
        let talk = Talk::new("t").add_slide(Slide::new("one"));
        assert!(TalkResponse::from(&talk).step_counts.is_empty());
    }
}
