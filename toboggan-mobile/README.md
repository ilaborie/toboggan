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
| `types` | `Talk`, `Slide`, `SlideKind`, `PresentationState`, `Command`, `ConnectionStatus`, `ClientRole`, `ErrorKind` |
| `deck` | Pairs slides with their step counts in one read (`deck_snapshot`) |
| `logging` | `LogSink` and `init_logging`, so the host app can show this crate's diagnostics |

The state enum is `PresentationState` and not `State` because names exported
here land in the host's own namespace, and `SwiftUI` and Compose each have a
`State` of their own — exported as `State` it shadowed theirs and the apps did
not compile.

`types` mirrors [`toboggan-core`](../toboggan-core) rather than re-exporting it:
UniFFI needs its own record and enum definitions, and the FFI surface should be
allowed to stay smaller and flatter than the domain model.

## The interface

```swift
let client = TobogganClient(
    // `retryDelay` is a `TimeInterval` — seconds, so this is one second.
    config: ClientConfig(url: "http://192.168.1.20:8080", maxRetries: 5, retryDelay: 1.0),
    clientName: "iPhone",
    handler: MyHandler()          // conforms to ClientNotificationHandler
)

client.connect()
client.sendCommand(command: .nextStep)

let state = client.getState()
let talk  = client.getTalk()
let deck  = client.getDeck()      // every slide, paired with its step count
```

`getDeck` reads the talk and the slides under one borrow. Assembling the deck by
asking `getSlide` for each index instead means indexing two independently
updated channels against each other, which shortens the deck when they skew.

Diagnostics from this crate go nowhere until the host installs a sink:

```swift
initLogging(sink: MyLogSink(), verbose: false)   // conforms to LogSink
```

The handler is called back on state changes, connection-status changes and
errors — which is how the app learns it has been disconnected, and how it learns
what role the server granted it.

`onError` carries an `ErrorKind`: `.server` is the server answering with a
complaint (a command refused for want of the presenter role), `.transport` is
not reaching it at all. They belong in different places in a UI, and telling them
apart by reading the message text does not survive a reworded string.

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
mise build:ios         # aarch64-apple-ios + simulator + the uniffi-bindgen binary
mise build:android     # via cargo-ndk, bindings included
```

`TobogganApp/TobogganApp/toboggan.swift`, `tobogganFFI.h` and `libtoboggan.a` are
**generated and git-ignored** — a fresh clone has none of them, so the Rust build
has to run before the Xcode project will open cleanly. Note that `mise build:ios`
builds the bindgen binary but does not *run* it; the Swift is produced by the
Xcode run-script phase (`TobogganApp/xc-universal-binary.sh`).

If you change anything exported across the boundary, regenerate — an edit to this
crate that does not is a mismatch, and `UniFFI`'s checksum guard turns it into a
`fatalError` at startup rather than a link error.

## License

MIT or Apache-2.0, at your option.

[uniffi]: https://github.com/mozilla/uniffi-rs
