# Toboggan for iOS

A presenter's remote for a running [Toboggan](../README.md) talk: the current
slide, its speaker notes, what comes next, and the controls to drive the deck —
on the device you are already holding.

SwiftUI, iOS 26, talking to the Rust core through [UniFFI](../toboggan-mobile).

## What it does

- Follows the talk live over the WebSocket protocol every other client speaks.
- Shows **speaker notes** for the current slide, as plain text: notes cross the
  FFI already flattened, because the phone has no HTML renderer.
- Shows the next slide's title, the position in the deck, and — when the deck
  plans timings — elapsed time and whether you are ahead of or behind the plan.
- Drives the deck: previous/next step, previous/next slide, blink, and
  tap-to-jump from the slide overview.
- Says plainly when the server has granted it **audience** rather than
  presenter, and disables the controls, instead of offering buttons that
  silently do nothing.

## Connecting

The app needs the address of the machine running `toboggan`, and — because a
phone is never that machine — a presenter token if it is to drive the deck.
Both come from one scan: open the talk's home page on the presenting machine and
point the app at the QR code it shows.

```bash
# on the presenting machine
toboggan -p ./my-talk --host 0.0.0.0 --presenter-token "$(uuidgen)"
```

Open `http://<that machine>:8080/?token=…` in a browser there; the page renders a
QR code carrying exactly that URL. Scan it from the app's Connection sheet.

Without a token the app still connects and follows along — it just watches. The
address and token can also be typed in by hand, and the token is kept in the
keychain rather than in preferences.

Everything the app logs is readable on the device from **Connection → Show log**,
which is the only way to see what went wrong when the phone is not plugged into
anything.

## Building

The Xcode project has a run-script phase that builds the Rust staticlib for the
current architecture and regenerates the Swift bindings from it, so an ordinary
build in Xcode is enough. Both artifacts are gitignored, so a fresh clone has
neither until that phase has run once.

```bash
mise build:ios     # the Rust side for device + both simulator architectures
mise test:ios      # xcodebuild test against a simulator
mise lint:ios      # SwiftLint
```

CI runs SwiftLint, a build, and the unit tests on every push.

## Layout

| Path | What lives there |
| --- | --- |
| `TobogganApp/App/` | `ContentView` — the scrolling deck view and its toolbar |
| `TobogganApp/Model/` | `PresentationModel` (the talk as this device sees it) and `TobogganSession` (the FFI handle) |
| `TobogganApp/Views/` | The content cards, the floating `RemoteBar`, the overview and log sheets |
| `TobogganApp/Connection/` | Settings, the keychain, the connection sheet, the QR scanner |
| `TobogganApp/Support/` | `AppLog` |
| `TobogganApp/toboggan.swift` | Generated UniFFI bindings — **do not edit** |

## Notes on the design

The deck is *content* and the controls are *chrome*. The slide title and notes
scroll underneath a floating glass bar and the system toolbar, which is what
gives Liquid Glass something to refract; the cards themselves are opaque, because
glass over glass reads as neither.

The server is authoritative for all navigation state. Nothing is updated
optimistically — a command this client is not allowed to send would otherwise
move the phone and not the projector.
