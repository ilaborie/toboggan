use serde::Serialize;
use serde::de::DeserializeOwned;
use toboggan_core::{Command, Notification, Slide, SlideId, SlidesResponse, TalkResponse};

#[derive(Debug, derive_more::Error, derive_more::From, derive_more::Display)]
pub enum TobogganApiError {
    ReqwestError(reqwest::Error),
}

#[derive(Debug, Clone)]
pub struct TobogganApi {
    client: reqwest::Client,
    api_url: String,
}

impl TobogganApi {
    pub fn new(api_url: impl Into<String>) -> Self {
        let api_url = api_url.into();
        let client = reqwest::Client::new();
        Self { client, api_url }
    }

    async fn get<T>(&self, path: &str) -> Result<T, TobogganApiError>
    where
        T: DeserializeOwned,
    {
        let url = format!(
            "{}/{}",
            self.api_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let response = self.client.get(&url).send().await?;
        let response = response.error_for_status()?;
        let result = response.json().await?;
        Ok(result)
    }

    async fn post<B, R>(&self, path: &str, body: &B) -> Result<R, TobogganApiError>
    where
        B: Serialize,
        R: DeserializeOwned,
    {
        let url = format!(
            "{}/{}",
            self.api_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let response = self.client.post(&url).json(body).send().await?;
        let response = response.error_for_status()?;
        let result = response.json().await?;
        Ok(result)
    }

    /// Fetches the talk information from the server.
    ///
    /// # Errors
    ///
    /// Returns `TobogganApiError` if the HTTP request fails or the response cannot be deserialized.
    pub async fn talk(&self) -> Result<TalkResponse, TobogganApiError> {
        self.get("/api/talk").await
    }

    /// Fetches all slides from the server.
    ///
    /// # Errors
    ///
    /// Returns `TobogganApiError` if the HTTP request fails or the response cannot be deserialized.
    pub async fn slides(&self) -> Result<SlidesResponse, TobogganApiError> {
        self.get("/api/slides").await
    }

    /// Fetches a specific slide by ID from the server.
    ///
    /// # Errors
    ///
    /// Returns `TobogganApiError` if the HTTP request fails or the response cannot be deserialized.
    pub async fn slide(&self, slide_id: SlideId) -> Result<Slide, TobogganApiError> {
        let path = format!("/api/slides/{slide_id}");
        self.get(&path).await
    }

    /// Sends a command to the server and returns the resulting notification.
    ///
    /// # Errors
    ///
    /// Returns `TobogganApiError` if the HTTP request fails or the response cannot be deserialized.
    pub async fn command(&self, command: Command) -> Result<Notification, TobogganApiError> {
        self.post("/api/command", &command).await
    }
}
