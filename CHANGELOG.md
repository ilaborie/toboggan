# Changelog

Notable changes to Toboggan. The format follows [Keep a Changelog], and the
project uses [Semantic Versioning] — while the major version is `0`, a minor
bump may break things.

Entries are grouped the way the commits are: this repository uses
[Conventional Commits], with `!` marking a breaking change.

## [Unreleased]

### Added

- **Mermaid diagrams in slides.** A ```` ```mermaid ```` fence is drawn to SVG while
  the deck builds — pure Rust, no Node and no headless browser — so the web
  client, the exported HTML, the PDF and the slide thumbnails all show the same
  picture with no script and no network, and a diagram that does not parse fails
  the build and names the slide rather than rendering as nothing in front of an
  audience. Tune one fence with `mermaid:theme=dark,width=60%` (`theme`,
  `background`, `width`, `nodeSpacing`, `rankSpacing`, `aspectRatio`,
  `maxLabelWidth`, `fastText`, `class`, `alt`; an unknown parameter is an
  error), and set deck-wide defaults with `--mermaid-config` or
  `[build] mermaid-config` in `toboggan.toml`, pointing at a JSON file in
  Mermaid's own config shape — where an unknown setting or a misspelled theme
  name is a build error too, rather than being quietly ignored. The background
  defaults to transparent so a diagram does not punch a white rectangle into a
  themed slide. `width` accepts only units CSS and Typst share (`%`, `pt`, `mm`,
  `cm`, `in`, `em`) so a deck cannot look right on the projector and then fail to
  export, and `background` takes only a real colour, so a misspelled one is a
  build error rather than a black rectangle. `class` and `alt` are free text;
  `class` styles the HTML only, and `alt` runs to the end of the fence so a
  label can contain commas — write it last.
- **A presenter view at `/presenter`** — the current slide beside the next one,
  the notes for where you are, and a status strip with the wall clock, an
  elapsed timer, deck progress, slide and reveal counters, and how far ahead or
  behind the deck's declared `duration` you are running. It is the same web
  application as `/run` with a different entry point, so it shares the socket,
  the keyboard and the state handling. `--open-presenter` opens both windows.
- **Presenter and audience roles.** A connection from the machine running the
  server presents; a connection from anywhere else presents only if it carries
  `--presenter-token`. `/api/terminal` and `/api/command` are gated on it. See
  [SECURITY.md](SECURITY.md).
- **A navigable static export.** `toboggan build -o deck.html` now ships a small
  inline script: arrow, space, `PageUp`/`PageDown` navigation, step reveals,
  `#slide-N` deep links and `f` for fullscreen. It was previously a scrolling
  stack with every step forced visible — which is what the GitHub Pages action
  published.
- **Presenter remotes work in every client.** `PageDown`, `PageUp` and
  `Backspace` are bound in the web, terminal and desktop clients; a physical
  clicker previously did nothing anywhere.
- **`f` fullscreen, `.`/`w` blank screen, and go-to-slide-by-number** in the web
  client. The terminal client's go-to is no longer capped at slide 9.
- **A deck can declare its language** (`lang` in `toboggan.toml` or `_cover.md`
  front matter), which reaches the `lang` attribute of every rendered page.
- **`--base-url`** for GitHub Pages sub-path deploys, wired through `action.yml`.
- **LaTeX math**: `$…$` and `$$…$$` render through KaTeX in HTML and pass
  through to Typst for PDF output.
- **Lint diagnostics carry the file they came from**, so `toboggan lint` can
  point at `slides/2_content/3-code.md:14` and render a miette snippet.
- **`toboggan lint --format {human,json,github,sarif}`** — `github` prints
  workflow commands that become annotations on the slide files themselves.
- **New lint rules**: `link/broken`, `structure/duplicate-slide-title`,
  `code/too-long`, `code/no-language`, `content/missing-notes` (opt-in), and
  `structure/over-budget` against a `[lint] max-duration`.
- **`toboggan build --list-themes`** is now reachable — the implementation
  existed but the flag was hardcoded to `false`.
- **`toboggan stats` reports the `duration` front matter**, which until now was
  parsed, validated, and read by nothing.
- **CI runs the project against its own decks**: both examples are linted and
  exported, the prebuilt guide artifact is checked for drift, and a Playwright
  smoke test drives every route in a browser. Rustdoc and `cargo machete` join
  the gate.
- **The guide is published** at <https://ilaborie.github.io/toboggan>.

### Changed

- **Slide front matter rejects unknown keys.** A typo like `classe = [...]`
  previously did nothing at all, silently.
- **`Content` answers "what does this slide say" once.** Nine copies of the same
  match had drifted into disagreement about whether `alt` or `raw` wins.
- Desktop shortcuts are described once and the help panel reads that description,
  instead of 28 hardcoded strings disconnected from the handlers.
- The exported HTML no longer fetches Google Fonts or KaTeX from a CDN, so a
  single-file deck is genuinely offline-capable.
- The workspace version is defined once in `[workspace.package]`.

### Fixed

- **The Python bindings' navigation is synchronous again.** `tbg.next()` pushed
  its command onto a channel and returned, while the resulting state only
  arrived a socket round trip later — so `tbg.next()` followed by `tbg.state`
  read the position the deck was in *before* the call, every time. Commands now
  travel over `POST /api/command`, which answers with the state it produced, so
  the deck has moved by the time the call returns and `example.py` needs none of
  the `sleep(1)` calls that were hiding this. A command the server refuses now
  raises — `PermissionError` for an audience connection, `RuntimeError` for a
  slide number the deck does not have — where before it did nothing at all and
  reported success. The socket stays for the job only it can do: reporting moves
  *other* clients made, and deck reloads.
- **The Python bindings release the GIL across network calls.** Connecting,
  waiting for registration and listing clients all blocked inside `block_on`
  while holding the interpreter lock, freezing every other Python thread for the
  duration — up to the five seconds registration is allowed to take.
- **One backtick no longer breaks the whole PDF.** Inline code containing a
  backtick was emitted with a `CommonMark`-style longer delimiter, which Typst
  does not implement — the span ran away to the end of the document and `typst`
  reported the failure on an unrelated line far below. The guide deck's own PDF
  and `GET /download.pdf` were both failing.
- **The presenter view's previews show the whole slide.** Both `zoom` rules
  targeted `.screen > *`, but the slide component attaches its shadow root to
  the host it is given, so the rules matched nothing and the next-slide pane
  showed a clipped horizontal slice of one line.
- **The dev tasks no longer bind to `0.0.0.0`.** `.mise.toml` exported
  `TOBOGGAN_HOST=0.0.0.0` for every task while `mise serve` printed
  `http://localhost:8080`.
- **The mise tasks and the scaffolded `SKILL.md` ran commands that no longer
  parse** after the positional path argument became `-p/--path`.
- **A deck folder resolves to one root** whether you pass `-p slides` or
  `-p ./slides/`; the two previously resolved code embeds and assets differently.
- **`toboggan.toml`'s template no longer advertises a rule id that does not
  exist** (`slide/too-many-words` → `content/excessive-words`).
- **Three panics across the UniFFI boundary** — a malformed URL, a runtime
  failure and an empty deck — return errors instead of aborting the host app.
- **The client library's REST half can reach the guarded endpoints.** Only the
  socket carried the presenter token, so `TobogganApi`'s `/api/command` and
  `/api/clients` were refused for every remote presenter however good their
  token; and `clients()` asked for a bare array where the endpoint answers with
  an object wrapping the list, so it failed to deserialize every response. The
  Python binding is the only caller, which is why neither had been noticed.
- **A silent RNG failure** no longer collapses reconnection jitter to zero,
  which had every client reconnecting in lockstep.
- **The PDF download filename** no longer comes from a second, divergent
  `slugify` that produced `my-great-talk-`.
- **An IO failure says what it was doing**; a *write* failure previously reported
  itself as a read failure with no path.
- **The web client stops stealing keys meant for an embedded terminal.**
- The GitHub Pages examples point at a tag and inputs that exist.

### Performance

- Each slide's HTML is parsed **once** per lint run rather than roughly 25 times,
  and once per stats pass rather than about 10.
- `/api/talk` and `/api/slides` no longer deep-clone the entire deck and
  recompute every slide's stats on every request.

### Documentation

- The root README, and every crate README, rewritten against what the code
  actually does — including a WebSocket protocol section that had listed
  `Play`/`Pause`/`Resume`, three commands that never existed.
- New: `CONTRIBUTING.md`, `SECURITY.md`, this file, and READMEs for `toboggan`,
  `toboggan-client`, `toboggan-stats`, `toboggan-tui` and `toboggan-desktop`.
- The guide deck gained slides on `toboggan.toml`, keyboard shortcuts, the quake
  terminal, images and assets, lint suppression, the terminal and desktop
  clients, the presenter view, and math.

### Removed

- References to `toboggan-esp32`, a client that no longer exists.
- `TODO.md`, which was entirely done.
- The legacy `toboggan-cli` binary, duplicated by the unified CLI.

## [0.1.0] - 2026-08-16

First release. The unified `toboggan` command, a `toboggan.toml` configuration
layer, a rioterm-based embedded terminal, and binaries for
`x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`.

[0.1.0]: https://github.com/ilaborie/toboggan/releases/tag/v0.1.0
[conventional commits]: https://www.conventionalcommits.org/en/v1.0.0/
[keep a changelog]: https://keepachangelog.com/en/1.1.0/
[semantic versioning]: https://semver.org/spec/v2.0.0.html
[unreleased]: https://github.com/ilaborie/toboggan/compare/v0.1.0...HEAD
