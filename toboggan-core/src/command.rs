use serde::{Deserialize, Serialize};

use crate::{ClientId, SlideId};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "command")]
pub enum Command {
    Register {
        name: String,
    },
    Unregister {
        client: ClientId,
    },
    Ping,
    // Navigation
    First,
    Last,
    GoTo {
        slide: SlideId,
    },
    #[serde(alias = "Next")]
    NextSlide,
    #[serde(alias = "Previous")]
    PreviousSlide,
    // Step navigation
    NextStep,
    PreviousStep,
    // Effect
    Blink,
}
