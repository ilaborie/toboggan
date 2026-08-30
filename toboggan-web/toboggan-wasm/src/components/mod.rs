use web_sys::HtmlElement;

/// The DOM the room looks at, shared by `/run` and the presenter's mirrors.
pub(crate) mod deck;

mod footer;
pub(crate) use self::footer::*;

mod slide;
pub(crate) use self::slide::*;

mod terminal;
pub(crate) use self::terminal::*;

mod quake_terminal;
pub(crate) use self::quake_terminal::*;

mod mirror;
pub(crate) use self::mirror::*;

mod shot;
pub(crate) use self::shot::*;

mod presenter;
pub(crate) use self::presenter::*;

/// The whole deck at a glance, searchable — the presenter view's, and mountable
/// anywhere else the deck is shown.
pub(crate) mod picker;
pub(crate) use self::picker::*;

/// Whether the deck's photographs are ready, shared by every surface made of
/// them.
pub(crate) mod thumbnails;

mod help;
pub(crate) use self::help::*;

mod toast;
pub(crate) use self::toast::*;

pub(crate) trait WasmElement {
    fn render(&mut self, host: &HtmlElement);
}
