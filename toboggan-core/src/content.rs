use std::fmt::Display;
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "<no content>"),
            Self::Text { text } => write!(f, "{text}"),
            Self::Html { raw, alt } => {
                if let Some(alt) = alt {
                    write!(f, "{alt}")
                } else {
                    write!(f, "{raw}")
                }
            }
            Self::IFrame { url } => write!(f, "{url}"),
            Self::Term { cwd, bootstrap } => {
                write!(f, "{}", cwd.display())?;
                if let Some(cmd) = bootstrap.last() {
                    write!(f, " - {cmd}")?;
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
                        write!(f, " - ")?;
                    }

                    write!(f, "{content}")?;
                }
                Ok(())
            }
        }
    }
}
