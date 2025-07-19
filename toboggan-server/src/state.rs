use std::collections::HashMap;
use std::sync::Arc;

use jiff::Timestamp;
use toboggan_core::{Slide, SlideId, Talk};

#[derive(Clone)]
pub struct TobogganState {
    pub(crate) started_at: Timestamp,
    pub(crate) talk: Arc<Talk>,
    pub(crate) slides: Arc<HashMap<SlideId, Slide>>,
    // TODO capture client sender
}

impl TobogganState {
    #[must_use]
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
}
