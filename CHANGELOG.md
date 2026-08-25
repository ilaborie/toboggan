# Changelog

Notable changes to Toboggan. The format follows [Keep a Changelog], and the
project uses [Semantic Versioning] — while the major version is `0`, a minor
bump may break things.

Entries are grouped the way the commits are: this repository uses
[Conventional Commits], with `!` marking a breaking change.

## [Unreleased]

## [0.1.1] - 2026-08-25

### Added

- **A deck can bring its own Typst preamble.** The generated one picks the
  touying theme and the aspect ratio, and neither can be taken back by anything
  written after them — so a deck could not choose another theme, or give itself
  more room. A `slides/_preamble.typ` now replaces it verbatim, as does
  `--typst-preamble <FILE>` (or `typst-preamble` under `[build]`, which wins
  over the deck file). This is the `_head.html` of the PDF side, except that it
  replaces rather than appends: a deck that sets it owns everything the
  generated preamble set up — the imports (touying, codly, codly-languages,
  gentle-clues, mitex), a theme show-rule that suppresses the theme's own
  heading display, without which every slide title prints twice
  (`subslide-preamble: none` for `themes.simple`, `header: none` for
  `themes.metropolis`), and `#show: codly-init.with()` with
  `#codly(languages: codly-languages)` for code fences. It travels on the
  `Talk`, so
  `toboggan pdf`, `toboggan build -o out.typ`, the server's `/download.pdf` and
  a prebuilt `talk.toml` all honour it. Thumbnails keep their own fixed-size,
  theme-less preamble, which is what makes a single slide render on one page.

- **State broadcasts carry a sequence number.** `Notification::State` and
  `Notification::TalkChange` now include `seq`, a counter the server advances
  once per change to the deck. A client that learns the state over only the
  WebSocket can ignore it — TCP already delivers those frames in order — but a
  client that *also* asks over `POST /api/command` gets two answers on two
  connections with nothing to say which is newer, and this is that. A server
  numbers every such frame or none of them, so `seq: 0` throughout is what a
  client talking to an older server sees — and it then behaves exactly as it did
  before.

- **`Toboggan` closes.** `close()` and the context-manager protocol, so the
  runtime shuts down deliberately with the GIL released rather than whenever the
  garbage collector gets to it while holding it.

- **`Slides` iterates and takes negative indices.** `for slide in client.slides`
  now type-checks (it always worked at runtime, invisibly to a checker), and
  `slides[-1]` is the last slide rather than an `OverflowError`.

- **`State.kind`** — `"init"`, `"running"` or `"done"` — so the three booleans
  can be asked as the one question they are.

- **The Python bindings are under test, in `mise` and in CI.** `toboggan-py` is
  the repository's third Cargo workspace, so the root `cargo` commands, `mise
  check:rust` and every CI job all went straight past it — long enough for two
  real defects to ship unnoticed. It now has `mise check:py` and `mise test:py`
  (both run by the top-level `mise check` and `mise test`) and a CI job of its
  own. The suite drives a real server on a free port: navigation must have
  landed by the time the call returns, a refused command must raise, a network
  call must not freeze other Python threads, and `toboggan_py.pyi` must still
  describe the module that was actually built — which is the one check nothing
  about a PyO3 build does for you.

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

- **BREAKING (Python): `State.is_first_slide` and `is_last_slide` are
  properties.** They took the deck's slide count as an argument, which made
  `state.is_first_slide(999)` a well-typed lie and forced callers to stitch
  together two reads a deck reload could take from different decks. `State`
  carries the count itself now, so `state.is_last_slide` replaces
  `state.is_last_slide(len(client.slides))`.

- **BREAKING (Python): `Slide.kind` is lower-case** — `"cover"`, `"part"`,
  `"standard"` — matching `hidden_in`, the front matter and serde. It came from
  `Debug` before, which made renaming a Rust enum variant a silent breaking
  change.

- **BREAKING (Python): `Talk.durations` is `float` seconds**, matching
  `Slide.duration`, and it and `step_counts` are always exactly as long as
  `titles`. They could previously be empty — meaning "not computed" rather than
  "none" — so `zip(talk.titles, talk.durations)` silently yielded nothing.

- **The bindings log instead of printing.** Eleven `println!`/`eprintln!` calls
  in an importable extension module went to the caller's stdout, where they
  corrupted any script whose output is data. They are `tracing` records now,
  bridged onto Python's own `logging` — silent by default, and available with
  `logging.basicConfig(level=...)` like anything else. The crate now carries the
  workspace's lint table, spelled out because it sits outside the workspace and
  inherits nothing; having no lint table at all — not even `print_stdout` — is
  why those calls compiled.

- **The Python type stub is checked, and the Python is linted.** `ruff`,
  `mypy` over a type-checked usage file, and `mypy.stubtest` all run in
  `mise check:py` and in CI. The crate's Python had no linter at all, and the
  600-line stub had nothing verifying its annotations — `for slide in
  client.slides` was an error against it while working perfectly at runtime.

### Fixed

- **A slide that does not fit no longer overflows in silence.** `#slide[..]` has
  no overflow handling: content that does not fit simply flows onto a second
  page, so a 23-slide deck could become 38 pages with nothing in the output
  saying so. Every emitted slide now carries invisible `#metadata` page markers,
  and `toboggan pdf` asks typst where they landed: it prints the deck's real
  page count and names each slide that spilled, with the pages it took. It stays
  a warning and exits 0 — a deck that does not fit is still a PDF — and
  `--no-overflow-check` skips the extra typst pass.

- **The PDF cover is the deck's cover again.** Rendering a Cover slide did
  nothing at all, on the grounds that its title and date were already emitted by
  the title slide — true of the title and date, and of nothing else. A cover
  whose point is a full-bleed illustration exported as a blank page with a date
  on it, with no warning. `_cover.md`'s body is now rendered under the title and
  date, its leading `# Title` stripped so it is not said twice.

- **The PDF no longer prints every slide title twice.** touying's `simple` theme
  displays the current level-2 heading above each slide, and the body emitted
  that same `== <title>` inside `#slide[..]` — so every content slide in the
  export carried its title twice, once from the theme and once from the body.
  The generated preamble now passes `subslide-preamble: none`, which is where it
  has to go: the theme stores the value and re-applies it per slide, so a later
  `config-common` would have been overwritten.

- **A socket that misses its connect budget still comes up.** The constructor
  bounded the handshake by wrapping `connect()` in a timeout, which cancels it —
  dropping the future before it has spawned anything, so nothing was left
  retrying and the socket was dead for the life of the client. It reported
  success anyway, and the getters went on answering from a cache that would
  never update again. `WebSocketClient::connect_within` bounds the handshake
  from the inside and hands expiry to the same retry loop a refused connection
  takes.

- **A deck rebuilt while the socket was down is no longer missed.** A server
  replays the current state on a new socket but not the `TalkChange` that was
  missed, so a client that reconnected went on serving the previous deck —
  coherent, wrong, and with nothing to say so. Reconnecting now refetches the
  deck, which is also what clears the staleness flag after a failed refetch;
  before, nothing did, and the error told the caller to retry something that
  could never succeed.

- **`state` no longer invents a position.** The cache started at `Init` and the
  constructor waited only for the role — which the server sends *before* the
  first state — so a running deck could be reported as not started, and stayed
  that way for good if the socket never came up. The cache now has no state
  until the server sends one, the constructor waits for both, and the getter
  raises rather than guessing.

- **An unreachable server is no longer reported as a bad shape.** `json()` fails
  the same way for a body that could not be read and one that could not be
  parsed, and both were mapped to the decode case — putting a connection reset
  mid-`/api/slides` back behind the `RuntimeError` that this release split the
  error type up to remove.

- **A collected client cannot freeze the interpreter.** Dropping a `Toboggan`
  without `close()` ran the runtime's blocking shutdown with the GIL held, and
  with logging bridged onto Python the worker threads reach for that same GIL on
  their way out. `Drop` now abandons the runtime instead of waiting for it;
  `close()` remains the way to wait deliberately, with the GIL released.

- **`Toboggan(...)` can no longer hang forever.** The socket connect had no
  timeout and ran *before* the bounded registration wait, so against a server
  that completes the TCP handshake and then never answers the upgrade, the
  constructor waited for a reply that was never coming — with the GIL released,
  so `Ctrl-C` only set a flag and the REPL had to be killed. The socket is
  bounded now, and `TobogganApi` carries connect and request timeouts of its own.

- **The Python bindings report *why* a call failed.** Every API error was one
  variant wrapping `reqwest::Error`, so a decode failure, a 403, a 500 and an
  unreachable server all arrived in Python as `ConnectionError` — which is how
  the `clients()` deserialization bug stayed hidden as long as it did. They are
  now told apart: a refusal keeps its status *and* the server's own explanation,
  a shape the client cannot read says so, and only a genuine transport failure
  is a `ConnectionError`.

- **A deck reload that fails no longer poisons the cache.** The refetch failure
  was logged and the new state committed anyway, against the *old* slides — so
  `state.slide` indexed into a deck that no longer existed. The last coherent
  snapshot is kept and marked stale, and the getters say so until a later reload
  succeeds.

- **`goto(0)` raises instead of moving to slide 1.** `saturating_sub` made the
  one number a caller carrying a 0-based index would actually pass the one
  number that moved the deck silently to the wrong place.

- **Checks that did not run no longer report success.** `mise test:py` exited 0
  when `uv` was missing and `mise check` then printed a green summary over a
  suite that had not run; pytest exits 0 when everything skipped, and a runner
  without a LAN address silently dropped the nine tests covering the whole
  token-and-role surface. `TOBOGGAN_PY_STRICT`, which CI sets, turns those
  missing preconditions into failures.

- **The Python bindings' navigation is synchronous.** `tbg.next()` pushed
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
  only caller of `clients()` is the Python binding, and `command()` had no
  callers anywhere in the workspace — which is why neither had been noticed.
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

[0.1.1]: https://github.com/ilaborie/toboggan/releases/tag/v0.1.1
[0.1.0]: https://github.com/ilaborie/toboggan/releases/tag/v0.1.0
[conventional commits]: https://www.conventionalcommits.org/en/v1.0.0/
[keep a changelog]: https://keepachangelog.com/en/1.1.0/
[semantic versioning]: https://semver.org/spec/v2.0.0.html
[unreleased]: https://github.com/ilaborie/toboggan/compare/v0.1.1...HEAD
