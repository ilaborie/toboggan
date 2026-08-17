use serde::{Deserialize, Serialize};

use crate::{Date, Slide};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Talk {
    pub title: String,
    pub date: Date,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
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
    pub slides: Vec<Slide>,
}

impl Talk {
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
    pub fn with_date(mut self, date: Date) -> Self {
        self.date = date;
        self
    }

    #[must_use]
    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    #[must_use]
    pub fn with_head(mut self, head: impl Into<String>) -> Self {
        self.head = Some(head.into());
        self
    }

    #[must_use]
    pub fn with_lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = Some(lang.into());
        self
    }

    #[must_use]
    pub fn with_default_terminal_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.default_terminal_cwd = Some(cwd.into());
        self
    }

    #[must_use]
    pub fn with_source_dir(mut self, dir: impl Into<String>) -> Self {
        self.source_dir = Some(dir.into());
        self
    }

    #[must_use]
    pub fn add_slide(mut self, slide: Slide) -> Self {
        self.slides.push(slide);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TalkResponse {
    pub title: String,
    pub date: Date,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    /// BCP 47 language tag; see [`Talk::lang`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    pub titles: Vec<String>,
    /// Step counts per slide (for clients that display step progress)
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
}
