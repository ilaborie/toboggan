use serde::{Deserialize, Serialize};

use crate::{Content, Date, Slide};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Talk {
    pub title: String,
    pub date: Date,
    #[serde(default)]
    pub footer: Content,
    pub slides: Vec<Slide>,
}

impl Talk {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        let date = Date::today();
        let slides = Vec::new();
        let footer = Content::default();

        Self {
            title,
            date,
            footer,
            slides,
        }
    }

    #[must_use]
    pub fn with_date(mut self, date: Date) -> Self {
        self.date = date;
        self
    }

    #[must_use]
    pub fn with_footer(mut self, footer: impl Into<Content>) -> Self {
        self.footer = footer.into();
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
    pub footer: Option<Content>,
    pub titles: Vec<String>,
}

impl From<Talk> for TalkResponse {
    fn from(value: Talk) -> Self {
        let Talk {
            title,
            date,
            footer,
            slides,
        } = value;
        let title = title.to_string();
        let footer = Some(footer);
        let titles = slides.iter().map(|it| it.title.to_string()).collect();

        Self {
            title,
            date,
            footer,
            titles,
        }
    }
}
