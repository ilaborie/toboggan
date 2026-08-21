# toboggan-mobile

The mobile client core: one Rust crate, exposed to Swift and Kotlin through
[UniFFI]. The connection, the reconnection, the protocol and the deck model live
here; the platform app only draws.

Both [`TobogganApp/`](../TobogganApp) (SwiftUI, iOS) and
[`toboggan-android/`](../toboggan-android) (Kotlin) are hosts for this crate.

## Modules

| Module | What it holds |
| --- | --- |
| `client` | `TobogganClient` and `ClientConfig` — the object the app drives |
| `handler` | `ClientNotificationHandler`, the callback interface the app implements |
| `types` | `Talk`, `Slide`, `SlideKind`, `State`, `Command`, `ConnectionStatus` |

`types` mirrors [`toboggan-core`](../toboggan-core) rather than re-exporting it:
UniFFI needs its own record and enum definitions, and the FFI surface should be
allowed to stay smaller and flatter than the domain model.

## The interface

```swift
let client = TobogganClient(
    config: ClientConfig(url: "http://192.168.1.20:8080", maxRetries: 5, retryDelay: 1000),
    clientName: "iPhone",
    handler: MyHandler()          // conforms to ClientNotificationHandler
)

client.connect()
client.sendCommand(command: .nextStep)

let state = client.getState()
let talk  = client.getTalk()
let slide = client.getSlide(index: 0)
```

The handler is called back on state changes, connection-status changes and
errors — which is how the app learns it has been disconnected, and how it learns
what role the server granted it.

### Presenting from a phone

A phone is not the machine running the server, so by default it can watch but not
navigate. Put the presenter token in the URL and it can drive the deck:

```
http://192.168.1.20:8080?token=s3cret
```

The token is split off the URL and offered during registration. See
[SECURITY.md](../SECURITY.md).

## Panics do not cross the boundary

A panic here unwinds into Objective-C or JNI and takes the host app down with it,
usually with nothing shown to the user. So the FFI surface does not panic: a
mistyped server address is passed through and fails as a connection error the
app already displays, rather than aborting in the constructor.

## Building

```bash
mise build:ios         # aarch64-apple-ios + simulator, xcframework, Swift bindings
mise build:android     # via cargo-ndk
```

`TobogganApp/TobogganApp/toboggan.swift` is **generated and checked in**. If you
change anything exported across the boundary, regenerate it — an edit to this
crate that does not is a mismatch that only shows up at link time.

## License

MIT or Apache-2.0, at your option.

[uniffi]: https://github.com/mozilla/uniffi-rs
