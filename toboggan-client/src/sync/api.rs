use serde::Serialize;
use serde::de::DeserializeOwned;
use toboggan_core::{Command, Notification, Slide, SlideId, SlidesResponse, TalkResponse};

#[derive(Debug, derive_more::Error, derive_more::From, derive_more::Display)]
pub enum TobogganApiError {
    UreqError(ureq::Error),
    IoError(std::io::Error),
}

#[derive(Debug, Clone)]
pub struct TobogganApi {
    agent: ureq::Agent,
    api_url: String,
}

impl TobogganApi {
    pub fn new(api_url: impl Into<String>) -> Self {
        let api_url = api_url.into();
        let agent = ureq::AgentBuilder::new()
            .timeout_read(std::time::Duration::from_secs(30))
            .timeout_write(std::time::Duration::from_secs(30))
            .build();
        Self { agent, api_url }
    }

    fn get<T>(&self, path: &str) -> Result<T, TobogganApiError>
    where
        T: DeserializeOwned,
    {
        let url = format!(
            "{}/{}",
            self.api_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        println!("🦀 ureq: Making GET request to {}", url);
        let response = self.agent.get(&url).call()?;
        let result = response.into_json()?;
        Ok(result)
    }

    fn post<B, R>(&self, path: &str, body: &B) -> Result<R, TobogganApiError>
    where
        B: Serialize,
        R: DeserializeOwned,
    {
        let url = format!(
            "{}/{}",
            self.api_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        println!("🦀 ureq: Making POST request to {}", url);
        let response = self.agent.post(&url).send_json(body)?;
        let result = response.into_json()?;
        Ok(result)
    }

    /// Fetches the talk information from the server.
    ///
    /// # Errors
    ///
    /// Returns `TobogganApiError` if the HTTP request fails or the response cannot be deserialized.
    pub fn talk(&self) -> Result<TalkResponse, TobogganApiError> {
        self.get("/api/talk")
    }

    /// Fetches all slides from the server.
    ///
    /// # Errors
    ///
    /// Returns `TobogganApiError` if the HTTP request fails or the response cannot be deserialized.
    pub fn slides(&self) -> Result<SlidesResponse, TobogganApiError> {
        self.get("/api/slides")
    }

    /// Fetches a specific slide by ID from the server.
    ///
    /// # Errors
    ///
    /// Returns `TobogganApiError` if the HTTP request fails or the response cannot be deserialized.
    pub fn slide(&self, slide_id: SlideId) -> Result<Slide, TobogganApiError> {
        let path = format!("/api/slides/{slide_id}");
        self.get(&path)
    }

    /// Sends a command to the server and returns the resulting notification.
    ///
    /// # Errors
    ///
    /// Returns `TobogganApiError` if the HTTP request fails or the response cannot be deserialized.
    pub fn command(&self, command: Command) -> Result<Notification, TobogganApiError> {
        self.post("/api/command", &command)
    }
}
