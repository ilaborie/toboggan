use web_sys::HtmlElement;

mod footer;
pub use self::footer::*;

mod navigation;
pub use self::navigation::*;

mod slide;
pub use self::slide::*;

mod toast;
pub use self::toast::*;

pub(crate) trait WasmElement {
    fn render(&mut self, host: &HtmlElement);
}
