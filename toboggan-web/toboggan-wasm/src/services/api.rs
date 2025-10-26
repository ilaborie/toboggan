use gloo::net::Error;
use gloo::net::http::Request;
use serde::de::DeserializeOwned;
use toboggan_core::{Slide, TalkResponse};

/// Client for interacting with the Toboggan API
#[derive(Debug, Clone)]
pub struct TobogganApi {
    api_base_url: String,
}

impl TobogganApi {
    /// Creates a new API client with the given base URL
    #[must_use]
    pub fn new(api_base_url: &str) -> Self {
        Self {
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
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
    pub async fn get_talk(&self) -> Result<TalkResponse, Error> {
        self.get("api/talk?footer=true").await
    }

    // /// Fetches all slides
    // pub async fn get_slides(&self) -> Result<Vec<Slide>, Error> {
    //     self.get("api/slides").await
    // }

    /// Fetches a specific slide by index
    pub async fn get_slide(&self, index: usize) -> Result<Slide, Error> {
        self.get(&format!("api/slides/{index}")).await
    }
}
