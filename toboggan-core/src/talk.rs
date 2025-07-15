use jiff::civil::DateTime;
use serde::{Deserialize, Serialize};

use crate::{Content, Slide};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Talk {
    pub title: Content,
    pub date: DateTime,
    pub slides: Vec<Slide>,
}
