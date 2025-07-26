# Changes to do

Ultrathink for that tasks.

Try to adhere to SOLID principle.

## main.ts
- introduce an enum for button id like `first-btn`
- when using `getRequiredElement` use the more generic element type (Open/Close principle)
- create constantes for `Command`
- `AppConfig should use env. var. with default fallback. See <https://vite.dev/guide/env-and-mode>
- try to reduce the size main.ts file by extracting code in sub-modules
- `AppConfig` should have one field `websocket: WebSocketConfig`

- Try to improve the WebSocketService naming. Maybe at some point we could change to use another kind of communication with the server (sse, webrtc, ...). Just find a better name.

## dom.ts

We should have a proper Errors component instead of `showError`
This component should handle the display/hide status of the current error.
It should be in a proper error.ts file.
And also display error in the console.

## websocket.ts

- Should send periodicaly a 'ping' command and compute the latency with the pong reply.

## General improvement

- Add a toast service that display errors, status changes (Running, Done, paused), and information like connecting/reconnecting the websocket.
- organize the code to isolate each component.


## Next step

Visual component should become web-component (custom element, shadow dom).
Do not use external libary to implements these web components.
