use gloo::net::Error;
use gloo::net::http::Request;
use serde::de::DeserializeOwned;
use toboggan_core::{OutlineResponse, Slide, SlideId, TalkResponse};

/// Client for interacting with the Toboggan API
#[derive(Debug, Clone)]
pub(crate) struct TobogganApi {
    api_base_url: String,
}

impl TobogganApi {
    /// Returns the API base URL
    #[must_use]
    pub(crate) fn base_url(&self) -> &str {
        &self.api_base_url
    }

    /// Creates a new API client with the given base URL
    #[must_use]
    pub(crate) fn new(api_base_url: &str) -> Self {
        Self {
            api_base_url: api_base_url.trim_end_matches('/').to_owned(),
        }
    }

    /// Makes a GET request to the specified path and deserializes the response
    async fn get<T>(&self, path: &str) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}/{}", self.api_base_url, path.trim_start_matches('/'));
        Request::get(&url).send().await?.json().await
    }

    /// Fetches the current talk
    pub(crate) async fn get_talk(&self) -> Result<TalkResponse, Error> {
        self.get("api/talk?footer=true&head=true").await
    }

    /// Fetches the deck as a searchable list of plain-text slides.
    ///
    /// Both pages ask, because both mount a slide picker: the response is every
    /// slide's body and its notes again, in plain text, which is most of the
    /// deck a second time and no use to a client that only shows one slide.
    pub(crate) async fn get_outline(&self) -> Result<OutlineResponse, Error> {
        self.get("api/outline").await
    }

    // /// Fetches all slides
    // pub async fn get_slides(&self) -> Result<Vec<Slide>, Error> {
    //     self.get("api/slides").await
    // }

    /// Fetches a specific slide by ID
    pub(crate) async fn get_slide(&self, slide_id: SlideId) -> Result<Slide, Error> {
        self.get(&format!("api/slides/{}", slide_id.index())).await
    }
}
