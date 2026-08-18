# toboggan-core

The domain model for [Toboggan](https://github.com/ilaborie/toboggan): what a
talk *is*, and what a client and a server say to each other about one.

This crate depends on nothing else in the workspace. The parser, the server, the
linter and every client are built on top of it, which is why it holds the wire
protocol too — `Command` and `Notification` have to mean the same thing on both
ends of the socket.

Not published to crates.io; it is used through the workspace.

## The model

| Type | What it is |
| --- | --- |
| `Talk` | A whole deck: title, date, language, footer, and its `Vec<Slide>` |
| `Slide` | One slide: `kind`, `title`, `body`, `notes`, terminals, `duration`, … |
| `SlideId` | A 0-based index, with `display_number()` for the 1-based one shown |
| `Content` | `Empty`, `Text { .. }`, or `Html { raw, alt, style }` |
| `SlideKind` | `Cover`, `Part`, or `Standard` |
| `State` | `Init`, `Running { current, current_step }`, or `Done { .. }` |
| `Command` | Client → server: navigation, registration, `Ping`, `Blink` |
| `Notification` | Server → clients: `State`, `Registered`, `Error`, `Pong`, … |
| `ClientRole` | `Presenter` or `Audience` — defaults to `Audience` |
| `TerminalConfig` | An embedded terminal declared by a slide |

```rust
use toboggan_core::{Content, Slide, SlideId, Talk};

let talk = Talk::new("Rust in Anger")
    .with_footer("RustFest")
    .add_slide(Slide::cover("Rust in Anger"))
    .add_slide(
        Slide::new("Why borrow?")
            .with_body(Content::html("<ul><li>No garbage collector</li></ul>"))
            .with_notes("Slow down here — this is the load-bearing idea."),
    );

assert_eq!(talk.slides.len(), 2);
assert_eq!(talk.slides[0].title.display_text(), "Rust in Anger");

// No `lang` set, so the deck is announced as English.
assert_eq!(talk.lang(), "en");

// Slides are indexed from zero and displayed from one.
assert_eq!(SlideId::new(0).display_number(), 1);
```

`Content::display_text()` is the one answer to "what does this slide say" —
plain text for `Text`, the `alt` (falling back to the raw HTML) for `Html`, and
an empty string for `Empty`. Use it rather than matching on the variants; nine
places used to do that by hand and had drifted into disagreeing about whether
`alt` or `raw` wins.

## The protocol

A client sends a `Command`; the server broadcasts a `Notification` to everyone
connected. Both are tagged enums, so the JSON is readable:

```rust
use toboggan_core::Command;

let json = r#"{"command":"Register","name":"tui"}"#;
let command = serde_json::from_str::<Command>(json).expect("a valid Register");

// Registration, deregistration and the heartbeat are the only commands an
// audience client may send. Everything else changes what the room sees.
assert!(!command.drives_the_deck());
assert!(Command::NextStep.drives_the_deck());
```

`drives_the_deck()` is deliberately written as a negation of the harmless
commands: a new variant is privileged until someone decides otherwise, rather
than slipping through a list of everything that was privileged on the day it was
written.

A client *offers* a token in `Register`; it never claims a role. The server
decides and reports the result in `Notification::Registered`.

## Features

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | ✅ | Enables `jiff/std` and `serde/std` |
| `tracing` | ✅ | Emits `tracing` events from the retry logic |
| `js` | | `jiff/js` and `getrandom/wasm_js`, for the wasm client |
| `openapi` | | Derives `utoipa::ToSchema` on the wire types |
| `test-utils` | | Test helpers for the other crates |

> [!NOTE]
> Despite the `std` feature, this crate is **not** `no_std`: there is no
> `#![no_std]` attribute and `--no-default-features` does not currently compile.
> Treat `std` as always-on.

## License

MIT or Apache-2.0, at your option.
