use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Display;
#[cfg(feature = "std")]
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    #[default]
    Empty,
    Text {
        text: String,
    },
    Html {
        raw: String,
        alt: Option<String>,
    },
    IFrame {
        url: String,
    },
    #[cfg(feature = "std")]
    Term {
        cwd: PathBuf,
        bootstrap: Vec<String>,
    },
    HBox {
        columns: String,
        contents: Vec<Content>,
    },
    VBox {
        rows: String,
        contents: Vec<Content>,
    },
    // TODO Image, Video
}

impl Display for Content {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => write!(fmt, "<no content>"),
            Self::Text { text } => write!(fmt, "{text}"),
            Self::Html { raw, alt } => {
                if let Some(alt) = alt {
                    write!(fmt, "{alt}")
                } else {
                    write!(fmt, "{raw}")
                }
            }
            Self::IFrame { url } => write!(fmt, "{url}"),
            #[cfg(feature = "std")]
            Self::Term { cwd, bootstrap } => {
                write!(fmt, "{}", cwd.display())?;
                if let Some(cmd) = bootstrap.last() {
                    write!(fmt, " - {cmd}")?;
                }
                Ok(())
            }
            Self::HBox {
                columns: _,
                contents,
            }
            | Self::VBox { rows: _, contents } => {
                let mut first = false;
                for content in contents {
                    if first {
                        first = false;
                    } else {
                        write!(fmt, " - ")?;
                    }

                    write!(fmt, "{content}")?;
                }
                Ok(())
            }
        }
    }
}
