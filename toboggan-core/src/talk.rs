use alloc::vec::Vec;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use crate::{Content, Slide};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Talk {
    pub title: Content,
    pub date: Date,
    pub slides: Vec<Slide>,
}
