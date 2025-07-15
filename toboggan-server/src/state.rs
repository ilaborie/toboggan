use std::collections::HashMap;
use std::sync::Arc;

use jiff::Timestamp;
use serde_json::json;
use toboggan_core::{Slide, SlideId, Talk};

#[derive(Clone)]
pub struct TobogganState {
    pub(crate) started_at: Timestamp,
    pub(crate) talk: Arc<Talk>,
    pub(crate) slides: Arc<HashMap<SlideId, Slide>>,
    // TODO capture client sender
}

impl TobogganState {
    pub fn new(talk: Talk) -> Self {
        let started = Timestamp::now();
        let talk = Arc::new(talk);

        let slides = talk
            .slides
            .iter()
            .map(|slide| (SlideId::next(), slide.clone()))
            .collect();
        let slides = Arc::new(slides);

        Self {
            started_at: started,
            talk,
            slides,
        }
    }

    pub fn health(&self) -> serde_json::Value {
        let elapsed = Timestamp::now() - self.started_at;

        json!({
            "status": "OK",
            "started_at": self.started_at,
            "elapsed": elapsed,
        })
    }
}
