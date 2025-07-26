use std::collections::HashMap;

use anyhow::Result;
use reqwest::Client;
use toboggan_core::{Slide, SlideId, Talk};
use tracing::info;

#[derive(serde::Deserialize)]
struct TalkResponse {
    talk: Talk,
}

#[derive(serde::Deserialize)]
struct SlidesResponse {
    slides: HashMap<SlideId, Slide>,
}

#[derive(Clone)]
pub struct SlideClient {
    client: Client,
    base_url: String,
}

impl SlideClient {
    pub fn new(base_url: &str) -> Self {
        let base_url = base_url
            .replace("ws://", "http://")
            .replace("wss://", "https://");
        let base_url = if base_url.ends_with("/api/ws") {
            base_url.replace("/api/ws", "")
        } else {
            base_url
        };

        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn fetch_talk(&self) -> Result<Talk> {
        let url = format!("{}/api/talk", self.base_url);
        info!("Fetching talk from: {}", url);

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch talk: {}",
                response.status()
            ));
        }

        let response_data: TalkResponse = response.json().await?;
        let talk = response_data.talk;
        info!("Fetched talk: {}", talk.title);
        Ok(talk)
    }

    pub async fn fetch_slides(&self) -> Result<HashMap<SlideId, Slide>> {
        let url = format!("{}/api/slides", self.base_url);
        info!("Fetching slides from: {}", url);

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch slides: {}",
                response.status()
            ));
        }

        let response_data: SlidesResponse = response.json().await?;
        let slide_map = response_data.slides;

        info!("Fetched {} slides", slide_map.len());
        Ok(slide_map)
    }

    #[allow(dead_code)]
    pub async fn fetch_slide(&self, slide_id: SlideId) -> Result<Slide> {
        let url = format!("{}/api/slides/{:?}", self.base_url, slide_id);
        info!("Fetching slide from: {}", url);

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch slide: {}",
                response.status()
            ));
        }

        let slide: Slide = response.json().await?;
        Ok(slide)
    }
}
