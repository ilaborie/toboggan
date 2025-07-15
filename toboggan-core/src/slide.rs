use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, LazyLock};

use serde::{Deserialize, Serialize};

use crate::Content;

static ID_SEQ: LazyLock<Arc<AtomicU8>> = LazyLock::new(Arc::default);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SlideId(u8);

impl SlideId {
    pub fn next() -> Self {
        let seq = LazyLock::force(&ID_SEQ);
        let id = seq.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Slide {
    pub kind: SlideKind,
    pub style: Style,
    pub title: Content,
    pub body: Content,
    pub notes: Content,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum SlideKind {
    Cover,
    Part,
    #[default]
    Standard,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Style(Vec<String>);
