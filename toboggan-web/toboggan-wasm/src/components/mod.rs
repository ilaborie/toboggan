use web_sys::HtmlElement;

mod footer;
pub(crate) use self::footer::*;

mod slide;
pub(crate) use self::slide::*;

mod terminal;
pub(crate) use self::terminal::*;

mod quake_terminal;
pub(crate) use self::quake_terminal::*;

mod presenter;
pub(crate) use self::presenter::*;

mod help;
pub(crate) use self::help::*;

mod toast;
pub(crate) use self::toast::*;

pub(crate) trait WasmElement {
    fn render(&mut self, host: &HtmlElement);
}
