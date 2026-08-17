use std::cell::Cell;

thread_local! {
    /// Whether an overlay currently owns the keyboard.
    static DECK_KEYS_CAPTURED: Cell<bool> = const { Cell::new(false) };
}

/// Takes the deck's keyboard bindings out of service while an overlay is up.
///
/// The quake terminal drops over the slide but only covers the top of the
/// viewport, so focus can sit outside it while it is open — on `<body>` after a
/// click on the slide behind, for instance. Whenever that happened, `space` and
/// the arrow keys reached the deck's global handler and advanced the
/// presentation *underneath* the shell the presenter was typing into; the slide
/// change then re-resolved the terminal's working directory and restarted the
/// session, throwing away their shell state mid-demo.
///
/// A shared flag rather than `stopPropagation` from the overlay's own listener:
/// the deck's handler and rioterm's textarea both sit on the path from `window`,
/// so stopping propagation early enough to protect the deck also stops the
/// keystrokes ever reaching the shell.
pub fn set_deck_keys_captured(captured: bool) {
    DECK_KEYS_CAPTURED.with(|flag| flag.set(captured));
}

/// Whether an overlay currently owns the keyboard.
#[must_use]
pub fn deck_keys_captured() -> bool {
    DECK_KEYS_CAPTURED.with(Cell::get)
}
