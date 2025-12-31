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
            slides,
        }
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
    pub titles: Vec<String>,
    /// Step counts per slide (for clients that display step progress)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub step_counts: Vec<usize>,
}

impl Default for TalkResponse {
    fn default() -> Self {
        Self {
            title: String::default(),
            date: Date::today(),
            footer: None,
            head: None,
            titles: vec![],
            step_counts: vec![],
        }
    }
}

impl From<Talk> for TalkResponse {
    fn from(value: Talk) -> Self {
        let Talk {
            title,
            date,
            footer,
            head,
            slides,
        } = value;
        let titles = slides.iter().map(|it| it.title.to_string()).collect();

        Self {
            title,
            date,
            footer,
            head,
            titles,
            step_counts: vec![], // Populated by server with SlideStats
        }
    }
}
