use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::{Content, Date, Slide};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Talk {
    pub title: Content,
    pub date: Date,
    pub slides: Vec<Slide>,
}

impl Talk {
    pub fn new(title: impl Into<Content>) -> Self {
        let title = title.into();
        let date = Date::today();
        let slides = Vec::new();

        Self {
            title,
            date,
            slides,
        }
    }

    #[must_use]
    pub fn with_date(mut self, date: Date) -> Self {
        self.date = date;
        self
    }

    #[must_use]
    pub fn add_slide(mut self, slide: Slide) -> Self {
        self.slides.push(slide);
        self
    }
}
