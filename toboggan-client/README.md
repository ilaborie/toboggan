# toboggan-client

The shared client library: it owns the socket, the reconnection, and the
dispatch, so that the terminal, desktop and mobile clients only have to decide
what to draw.

Not a binary. Used by [`toboggan-tui`](../toboggan-tui),
[`toboggan-desktop`](../toboggan-desktop) and
[`toboggan-mobile`](../toboggan-mobile). The web client speaks the same protocol
but has its own wasm implementation.

## What it gives you

| Type | Role |
| --- | --- |
| `TobogganApi` | The REST side: fetch the talk, the slides, one slide |
| `WebSocketClient` | The live side: send `Command`s, receive `Notification`s |
| `TobogganClientCore<H>` | The two together, driving a `NotificationHandler` |
| `NotificationHandler` | The trait a client implements to react to the deck |
| `CommunicationMessage` | What the socket reports upward, including `Registered` |
| `ConnectionStatus` | Connected, reconnecting, disconnected |
| `TobogganConfig` | Host, port, and an optional presenter token |

```rust,ignore
use toboggan_client::TobogganConfig;

let config = TobogganConfig::new("localhost", 8080)
    // Only needed when the server is on another machine: a client on the
    // server's own machine always presents.
    .with_presenter_token(Some("s3cret"));
```

## Reconnection

The socket reconnects on its own, with exponential backoff and jitter. The jitter
matters more than it looks: without it, every client in a room that lost wifi
comes back at the same instant and hits the server together.

`ConnectionStatus` is surfaced rather than hidden, because a presenter needs to
know their clicker has stopped working before they press it.

## Roles

A client sends its token, if it has one, in `Command::Register`. It never claims
a role — the server decides and reports the result in
`CommunicationMessage::Registered`. A client that connects across the network
without a token can watch, but its navigation commands are refused; see
[SECURITY.md](../SECURITY.md).

## License

MIT or Apache-2.0, at your option.
