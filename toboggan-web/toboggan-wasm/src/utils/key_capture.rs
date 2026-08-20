use std::cell::{Cell, RefCell};

use gloo::console::error;
use gloo::utils::document;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{AddEventListenerOptions, Event, EventTarget, HtmlElement};

use crate::{blur_active_element, deepest_active_element, install_capture_keydown};

/// Worn by the terminal window currently holding the keyboard.
///
/// Styled as an outline, which takes no space — a ring that reflowed the window
/// would change the box the grid is measured from and resize the shell.
const HAS_KEYS_CLASS: &str = "terminal-has-keys";

/// Hands the keyboard back without touching the shell's own bindings.
///
/// Bare `Escape` would be the obvious choice and is deliberately not used: a
/// terminal on a slide exists to run `vim`, `less` and friends, all of which
/// need it. `Shift` is never meaningful to a shell in this position.
const RELEASE_KEY: &str = "Escape";

/// Identifies one widget's claim on the keyboard.
///
/// Handed out per terminal so a release can be matched against the claim that
/// made it: a terminal being torn down must not take the keyboard away from
/// whichever one has since taken it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardOwner(u32);

/// The live claim: who holds the keyboard, and the window wearing the ring.
struct KeyboardClaim {
    owner: KeyboardOwner,
    window: HtmlElement,
}

thread_local! {
    static NEXT_OWNER: Cell<u32> = const { Cell::new(0) };
    /// One slot: claiming evicts whoever held it, so two terminals can never
    /// both believe they own the keyboard.
    static CLAIM: RefCell<Option<KeyboardClaim>> = const { RefCell::new(None) };
}

/// Reserves an owner id for a widget that may take the keyboard.
#[must_use]
pub fn next_keyboard_owner() -> KeyboardOwner {
    NEXT_OWNER.with(|next| {
        let id = next.get();
        next.set(id.wrapping_add(1));
        KeyboardOwner(id)
    })
}

/// Takes the deck's keyboard bindings out of service in favour of `window`.
///
/// A flag rather than `stopPropagation` from the terminal's own listener: the
/// deck's handler and rioterm's hidden textarea both sit on the path down from
/// `window`, so stopping propagation early enough to protect the deck also stops
/// the keystrokes ever reaching the shell.
///
/// A flag rather than a focus test, too. Focus is where this used to live, and
/// it does not survive contact with a presentation: rioterm focuses its textarea
/// only from a `mousedown` on its own canvas, so clicking the title bar, a
/// traffic light or the padding left the terminal looking like the thing being
/// typed into while `space` still advanced the deck.
pub fn claim_keyboard(owner: KeyboardOwner, window: &HtmlElement) {
    CLAIM.with_borrow_mut(|claim| {
        if let Some(previous) = claim.take()
            && previous.owner != owner
        {
            let _ = previous.window.class_list().remove_1(HAS_KEYS_CLASS);
        }
        let _ = window.class_list().add_1(HAS_KEYS_CLASS);
        *claim = Some(KeyboardClaim {
            owner,
            window: window.clone(),
        });
    });
}

/// Gives the deck its keys back, if `owner` is still the one holding them.
///
/// Focus is only taken back when it is still inside the claimed window. A
/// release triggered by a click elsewhere has already moved focus to whatever
/// was clicked, and blurring that would be this function undoing someone else's
/// work.
pub fn release_keyboard(owner: KeyboardOwner) {
    let held = CLAIM.with_borrow_mut(|claim| {
        if claim.as_ref().is_none_or(|held| held.owner != owner) {
            return None;
        }
        claim.take()
    });
    let Some(held) = held else {
        return;
    };
    let _ = held.window.class_list().remove_1(HAS_KEYS_CLASS);
    if deepest_active_element().is_some_and(|active| held.window.contains(Some(active.as_ref()))) {
        blur_active_element();
    }
}

/// Whether a terminal currently owns the keyboard.
#[must_use]
pub fn deck_keys_captured() -> bool {
    CLAIM.with_borrow(Option::is_some)
}

/// Installs the two page-lifetime ways to hand the keyboard back.
///
/// Both live here rather than on the terminal: they have to work for whichever
/// terminal holds the claim, and one listener that reads the claim is cheaper
/// than one per terminal per slide — a document-level listener registered by a
/// slide's terminal would outlive the slide.
pub fn install_keyboard_release() {
    install_release_on_outside_click();
    install_release_key();
}

/// Releases the keyboard when a click lands outside the claimed window.
///
/// `composedPath` rather than `target`, because a click inside the terminal is
/// retargeted to its shadow host; and the claimed window rather than
/// `document.activeElement`, which retargets all the way to the *outermost*
/// host — the slide — and so counts a click anywhere on the slide as inside.
fn install_release_on_outside_click() {
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        let Some((owner, window)) =
            CLAIM.with_borrow(|claim| claim.as_ref().map(|held| (held.owner, held.window.clone())))
        else {
            return;
        };
        let window: &EventTarget = window.as_ref();
        let inside = event
            .composed_path()
            .iter()
            .any(|node| node.dyn_ref::<EventTarget>() == Some(window));
        if !inside {
            release_keyboard(owner);
        }
    });

    let options = AddEventListenerOptions::new();
    options.set_capture(true);
    if document()
        .add_event_listener_with_callback_and_add_event_listener_options(
            "click",
            closure.as_ref().unchecked_ref(),
            &options,
        )
        .is_err()
    {
        error!("Failed to register keyboard-release listener");
    }
    closure.forget();
}

/// Releases the keyboard on `Shift`+`Escape`.
///
/// Capture phase, and consumed outright, so the shell never sees it: the whole
/// point is a chord that a presenter without a mouse can use to get the deck's
/// keys back without `Escape` itself becoming unusable inside the terminal.
fn install_release_key() {
    install_capture_keydown(move |event| {
        if event.key() != RELEASE_KEY || !event.shift_key() {
            return;
        }
        let Some(owner) = CLAIM.with_borrow(|claim| claim.as_ref().map(|held| held.owner)) else {
            return;
        };
        event.prevent_default();
        event.stop_propagation();
        release_keyboard(owner);
    });
}
