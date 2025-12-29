use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::Style;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type")]
pub enum Content {
    #[default]
    Empty,
    Text {
        text: String,
    },
    Html {
        raw: String,
        #[serde(default, skip_serializing_if = "Style::is_default")]
        style: Style,
        #[serde(skip_serializing_if = "Option::is_none")]
        alt: Option<String>,
    },
}

impl Content {
    pub(crate) fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn text(text: impl Into<String>) -> Self {
        let text = text.into();
        Self::Text { text }
    }

    pub fn html(raw: impl Into<String>) -> Self {
        let style = Style::default();
        let raw = raw.into();
        let alt = None;
        Self::Html { raw, alt, style }
    }

    pub fn html_with_alt(raw: impl Into<String>, alt: impl Into<String>) -> Self {
        let style = Style::default();
        let raw = raw.into();
        let alt = Some(alt.into());
        Self::Html { raw, alt, style }
    }
}

impl From<&str> for Content {
    fn from(text: &str) -> Self {
        Self::text(text)
    }
}

impl From<String> for Content {
    fn from(text: String) -> Self {
        Self::text(text)
    }
}

impl Display for Content {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => write!(fmt, "<no content>"),
            Self::Text { text } => write!(fmt, "{text}"),
            Self::Html { raw, alt, .. } => {
                if let Some(alt) = alt {
                    write!(fmt, "{alt}")
                } else {
                    write!(fmt, "{raw}")
                }
            }
        }
    }
}
