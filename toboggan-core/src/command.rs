use serde::{Deserialize, Serialize};

use crate::{ClientId, SlideId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "command")]
pub enum Command {
    Register {
        name: String,
        /// The presenter token this client was given, if any.
        ///
        /// A client *offers* a token; it never claims a role. The server is the
        /// only thing that decides what a connection may do, so a client that
        /// lies about itself gains nothing.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token: Option<String>,
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

impl Command {
    /// Whether obeying this command would change what the room sees.
    ///
    /// The gate for [`crate::ClientRole::Audience`]. Written as a negation of
    /// the harmless commands on purpose: a new variant is privileged until
    /// someone decides otherwise, rather than slipping through a list of
    /// everything that was privileged on the day it was written.
    #[must_use]
    pub const fn drives_the_deck(&self) -> bool {
        !matches!(
            self,
            Self::Register { .. } | Self::Unregister { .. } | Self::Ping
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// A client that omits the token is an ordinary audience member, not a
    /// parse error — the field was added after the protocol shipped.
    #[test]
    fn a_register_without_a_token_still_parses() {
        let command = serde_json::from_str::<Command>(r#"{"command":"Register","name":"tui"}"#)
            .expect("Register with no token");
        assert_eq!(
            command,
            Command::Register {
                name: "tui".to_owned(),
                token: None,
            }
        );
    }

    #[test]
    fn only_the_handshake_and_the_heartbeat_are_unprivileged() {
        for harmless in [
            Command::Register {
                name: "x".to_owned(),
                token: None,
            },
            Command::Unregister {
                client: ClientId::from_key(slotmap::DefaultKey::default()),
            },
            Command::Ping,
        ] {
            assert!(!harmless.drives_the_deck(), "{harmless:?}");
        }
        for privileged in [
            Command::First,
            Command::Last,
            Command::GoTo {
                slide: SlideId::FIRST,
            },
            Command::NextSlide,
            Command::PreviousSlide,
            Command::NextStep,
            Command::PreviousStep,
            Command::Blink,
        ] {
            assert!(privileged.drives_the_deck(), "{privileged:?}");
        }
    }
}
