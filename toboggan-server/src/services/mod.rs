mod client;
pub use self::client::*;

mod talk;
pub use self::talk::*;

mod shots;
pub use self::shots::{ShotFailure, ShotOptions, find_browser, shoot_slides};

mod overview;
pub use self::overview::{Drawn, OverviewOptions, ThumbnailRenderer, generate_overview};

mod thumbnails;
pub(crate) use self::thumbnails::{AssetLookup, ThumbStatus, ThumbnailService};

// mod client_service;
// mod talk_service;

// pub use client_repository::*;
// pub use client_service::*;
// pub use talk_service::*;
