+++
title = "Multi-client sync"
classes = ["no_title", "wide"]
+++

# Everyone on the same slide

The server keeps a single presentation **state machine** and pushes it to every
connected client over a WebSocket.

<!-- pause -->

- Navigate on one device → all clients follow, instantly
- Late joiners receive the **current** slide on connect
- Reconnects re-sync automatically

<!-- pause -->

### Endpoints worth knowing

| Path | Purpose |
|---|---|
| `/` | The web client (this UI) |
| `/api/ws` | Presentation sync WebSocket |
| `/api/terminal` | Embedded-terminal WebSocket |
| `/api/talk`, `/api/slides` | REST views of the deck |
| `/doc` | Interactive OpenAPI docs |

<!-- notes -->
State transitions: Init → Running ⇄ Paused → Done. Commands (Next, Previous,
GoTo, …) arrive over /api/ws and the new state is broadcast to all clients.
