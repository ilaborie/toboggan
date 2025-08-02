use serde::{Deserialize, Serialize};
use toboggan_core::{Content, Date, Talk};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TalkResponse {
    /// The title of the presentation.
    title: Content,

    /// The date of the presentation.
    date: Date,

    /// The slides titles
    titles: Vec<String>,
}

impl From<Talk> for TalkResponse {
    fn from(value: Talk) -> Self {
        let Talk {
            title,
            date,
            slides,
        } = value;
        let titles = slides.iter().map(|it| it.title.to_string()).collect();

        Self {
            title,
            date,
            titles,
        }
    }
}
