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

    fn build_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.api_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    async fn get<T>(&self, path: &str) -> Result<T, TobogganApiError>
    where
        T: DeserializeOwned,
    {
        let url = self.build_url(path);
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
        let url = self.build_url(path);
        let response = self.client.post(&url).json(body).send().await?;
        let response = response.error_for_status()?;
        let result = response.json().await?;
        Ok(result)
    }

    pub async fn talk(&self) -> Result<TalkResponse, TobogganApiError> {
        self.get("/api/talk").await
    }

    pub async fn slides(&self) -> Result<SlidesResponse, TobogganApiError> {
        self.get("/api/slides").await
    }

    pub async fn slide(&self, slide_id: SlideId) -> Result<Slide, TobogganApiError> {
        let path = format!("/api/slides/{}", slide_id.index());
        self.get(&path).await
    }

    pub async fn command(&self, command: Command) -> Result<Notification, TobogganApiError> {
        self.post("/api/command", &command).await
    }
}
