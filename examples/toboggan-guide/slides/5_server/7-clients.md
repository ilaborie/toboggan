+++
title = "Other clients"
classes = ["no_title", "wide"]
+++

# The same deck, elsewhere

The browser is not the only client. Against a running server:

```console
$ toboggan tui        # a terminal
$ toboggan desktop    # a native window
```

<!-- pause -->

Everything stays in sync — press `→` in the terminal and the projector moves,
because the deck is shared state rather than each client's own view.

| | Notes | Next slide | Timer | Terminals |
|---|:-:|:-:|:-:|:-:|
| `/run` | — | — | — | ✅ |
| `/presenter` | ✅ | ✅ | ✅ | — |
| `toboggan tui` | ✅ | ✅ | — | — |
| `toboggan desktop` | ✅ | — | — | — |

> [!WARNING]
> A client on another machine can watch, but not navigate, and cannot open a
> terminal. Give it `--presenter-token` to let it drive.

<!-- notes -->
The terminal client is genuinely useful over ssh, and it was the presenter view
before `/presenter` existed.

The mobile clients speak the same protocol through the `toboggan-mobile` crate,
which is the same Rust talking to Swift and Kotlin over UniFFI.
