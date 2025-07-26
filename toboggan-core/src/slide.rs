#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU8, Ordering};

#[cfg(feature = "alloc")]
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::Content;

#[cfg(feature = "alloc")]
static ID_SEQ: Lazy<Arc<AtomicU8>> = Lazy::new(Arc::default);

#[cfg(not(feature = "alloc"))]
static ID_SEQ: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SlideId(u8);

impl SlideId {
    #[must_use]
    pub fn next() -> Self {
        #[cfg(feature = "alloc")]
        {
            let seq = Lazy::force(&ID_SEQ);
            let id = seq.fetch_add(1, Ordering::Relaxed);
            Self(id)
        }
        #[cfg(not(feature = "alloc"))]
        {
            let id = ID_SEQ.fetch_add(1, Ordering::Relaxed);
            Self(id)
        }
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
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum SlideKind {
    Cover,
    Part,
    #[default]
    Standard,
}

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Style(Vec<String>);

#[cfg(not(feature = "alloc"))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Style;
