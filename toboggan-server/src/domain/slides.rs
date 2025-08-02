use serde::Serialize;

use toboggan_core::Slide;

#[derive(Debug, Clone, Serialize)]
pub struct SlidesResponse {
    pub(crate) slides: Vec<Slide>,
}
