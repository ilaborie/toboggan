use serde::{Deserialize, Serialize};
use toboggan_core::Slide;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SlidesResponse {
    pub(crate) slides: Vec<Slide>,
}
