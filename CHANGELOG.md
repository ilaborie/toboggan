# Changelog

Notable changes to Toboggan. The format follows [Keep a Changelog], and the
project uses [Semantic Versioning] — while the major version is `0`, a minor
bump may break things.

Entries are grouped the way the commits are: this repository uses
[Conventional Commits], with `!` marking a breaking change.

## [Unreleased]

### Added

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
