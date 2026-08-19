# PR #55 review fixes

Plan: `~/.claude/plans/logical-jingling-thunder.md`
Branch: `feat/review-program` (everything lands here before #55 merges)

## A. Critical

- [x] A1 — Reject cross-origin requests to privileged routes (`router/origin.rs`)
- [x] A2 — Stop `copy_dir` recursing into its own destination
- [x] A3 — `Secret` newtype, one definition of a valid token
- [x] A4 — `toboggan mcp` emits `--dir`, accepts only `--path`
- [x] A5 — Prune the theme list; error instead of panicking
- [x] A6 — Escape the alert title in the Typst output

## B. Presenter view and web client

- [x] B1 — Log each presenter selector miss; keep the panes that resolved
- [x] B2 — Mount the toast under `body` on the presenter branch
- [x] B3 — Guard `await init()`; render the failure into the page
- [x] B4 — `navigate.js`: real catch + modifier guard
- [x] B5 — Report rejected font faces; log env coercion
- [x] B6 — Expire `pending_goto`; modifier guard; reject leading `0`

## C. Token UX

- [x] C2 — Make the three token decoders agree
- [x] C3 — Page links carry `?token=`
- [x] C4 — `report_access_posture` prints the presenter URL
- [x] C5 — TUI/desktop read the granted role

## D. Types and escaping

- [x] D1 — `Presenter(())`; drop `Ord` from `ClientRole`
- [x] D2 — `escape_attribute`; escape `style`/`classes`
- [ ] D3 — `TalkResponse` → `Vec<SlideSummary>`
- [x] D4 — `OnceCell` → `OnceLock` in `RuleContext`
- [x] D5 — `with_source_path` `pub(crate)`; drop stored `display_number`

## E. Lint, build, CLI

- [x] E1 — Terminal rules: key on `(cwd, cmd)`; fix `unresolved-cwd` + its test; DRY test helpers
- [ ] E2 — Normalise `--base-url` trailing slash
- [x] E3 — Drop the `source_dir/public` candidate
- [ ] E4 — `build.rs` gates on `presenter.html`
- [ ] E5 — Lint stdout through `WriteStdout` (EPIPE)
- [ ] E6 — `examples/slide.md` `css` → `style`; part `duration` dropped
- [ ] E7 — Desktop fullscreen: implement or remove

## F. Tests

- [x] F1 — Router tests with a configured token
- [x] F2 — Cross-origin refusal test
- [ ] F3 — WS audience-refusal test
- [x] F4 — Fenced code block containing backticks
- [ ] F5 — `all_rules()` registration + severity table
- [x] F6 — First wasm tests + CI step
- [x] F7 — `layout.spec.ts` serial + `expect.poll`
- [ ] F8 — Desktop `command()`; TUI key bindings
- [ ] F9 — `HtmlDocument` unit tests
- [ ] F10 — `expect()` over `unwrap()` in `core/config.rs` tests

## G. Documentation

- [x] G1 — Delete the stale Typst doc line
- [~] G2 — Security wording: constant-time, loopback, `ssh -L` (auth.rs done; SECURITY.md pending)
- [ ] G3 — README protocol frames; `examples/README.md` commands
- [ ] G4 — Dead mechanisms: retry backoff, `connection_timeouts`, `hidden_in`
- [ ] G5 — Long tail of factual corrections
- [ ] G6 — Workspace/`mise check` claims

## H. Pre-existing (not regressions from #55)

- [ ] H1 — Mid-session reconnection is dead code
- [ ] H2 — `inject_head_html` loops forever on error

## Review

_To be filled in as work lands._
