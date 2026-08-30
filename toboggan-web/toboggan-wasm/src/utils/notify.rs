//! Telling the speaker something, from anywhere on the page.
//!
//! The toast belongs to [`crate::App`], which mounts it and owns it. Most things
//! that need it are a message away from it — `Session::toast` is right there.
//! Components are not: a picker three shadow roots down has no route to it, and
//! handing every component a channel is a great deal of plumbing for a message
//! that is always the same two values.
//!
//! So the app registers a sink once and anything may write to it. Deliberately
//! narrow: this is for failures the *user* has to know about, the ones where
//! something they just did produced no visible effect. Everything else belongs
//! in the console, where [`gloo::console::error`] already puts it.

use std::cell::RefCell;
use std::rc::Rc;

use gloo::console::error;

use crate::ToastType;

/// What a message costs to send: nothing, if nobody is listening yet.
pub(crate) type Sink = Rc<dyn Fn(ToastType, &str)>;

thread_local! {
    /// Set once, by `App::render`. `None` before that, and on the two entry
    /// points that mount no app at all — the mirror and the shot page, neither
    /// of which has a speaker looking at it.
    static SINK: RefCell<Option<Sink>> = const { RefCell::new(None) };
}

/// Points [`notify`] at this page's toast.
pub(crate) fn set_notifier(sink: Sink) {
    SINK.with_borrow_mut(|slot| *slot = Some(sink));
}

/// Says something to whoever is looking at this page.
///
/// Falls back to the console when there is nowhere to say it, rather than
/// dropping the message: a page with no toast is exactly the page where a
/// swallowed failure is hardest to find.
pub(crate) fn notify(kind: ToastType, message: &str) {
    let sink = SINK.with_borrow(Clone::clone);
    match sink {
        Some(sink) => sink(kind, message),
        None => error!("Nowhere to show a message:", message),
    }
}
