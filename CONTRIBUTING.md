# Contributing to Toboggan

Thanks for looking. Bug reports, slides that break the parser, and pull requests
are all welcome.

## Before you push

> [!IMPORTANT]
> The repository's `prek` checks are installed as **git** hooks
> (`.git/hooks/pre-commit`, `pre-push`). This project is developed with
> [jj](https://jj-vcs.github.io/jj/), **which does not run git hooks** — so if
> you use jj, nothing checks your work locally unless you ask it to.

Run this yourself, whichever VCS you use:

```bash
mise check     # fmt + clippy + nextest — the root workspace only
```

CI re-runs the same gate on every push and pull request regardless, so a skipped
local check costs you a round trip rather than landing broken code.

## Setting up

```bash
git clone https://github.com/ilaborie/toboggan
cd toboggan
mise install       # Rust tooling, wasm-pack, nextest, swiftlint, …
mise build:web     # wasm-pack + pnpm → toboggan-web/dist
cargo build
```

`toboggan-server` embeds `toboggan-web/dist` at compile time via `rust-embed`,
and its `build.rs` **fails when that folder is missing**. So `mise build:web`
comes before your first `cargo build`, and again whenever you change anything
under `toboggan-web/`. The bundle is a gitignored build artifact; it is never
committed.

## Three Cargo workspaces

The embedded web client is a separate workspace, rooted at
`toboggan-web/Cargo.toml` with `toboggan-wasm` as its only member. Neither
`--workspace` nor `--all-targets` from the repository root reaches it, and
neither does `cargo nextest run` — **nor does `mise check`**, which runs
`check:rust` at the root. CI covers it in a job of its own. If you touch it:

```bash
cd toboggan-web/toboggan-wasm
cargo +nightly fmt --all --check
cargo clippy --all-targets --target wasm32-unknown-unknown -- -D warnings
```

It is linted against `wasm32-unknown-unknown` because that is the only target it
is ever built for; a host-target lint would be checking code the browser never
runs. CI has a dedicated job for exactly this.

`toboggan-py` is the third, and has the same blind spot for the same reason —
it is a `cdylib` built by maturin, so it is in the root manifest's `exclude`
list. It went uncovered long enough for two real defects to ship: a `lib.rs`
`cargo fmt` had never seen, and a `TobogganApi::clients()` that failed to
deserialize on *every* call, unnoticed because the bindings are its only caller.
It now has its own mise lane and its own CI job:

```bash
mise check:py   # cargo fmt + clippy, ruff, mypy, stubtest, and the test suite
mise test:py    # just the tests
```

Its tests start a real server on a free port and drive it, so they need `uv`
(declared in `.mise.toml`) and either a prebuilt `toboggan` in `TOBOGGAN_BIN` or
a working `cargo run`. `mise check` runs this lane along with the others.

Both tasks **fail** rather than skip without `uv`. That is deliberate: they used
to exit 0, so `mise check` printed a green Python summary over a suite that had
not run — and the point of this lane is that the crate stops being invisible.

The Python side is linted with `ruff` and the hand-written `toboggan_py.pyi` is
checked twice: `mypy` over `tests/typing/usage.py`, which exercises every
documented call so the stub's *annotations* are verified rather than just its
names, and `mypy.stubtest`, which compares the stub against the built module.
When you change the Python API, change the stub and add the new call to
`usage.py` — a promise no caller has written down is a promise nobody checks.

## Code guidelines

- **Edition 2024**, MSRV 1.95. `cargo +nightly fmt` — the repo's `rustfmt.toml`
  uses nightly-only options, so stable `cargo fmt` will disagree with CI.
- **No warnings.** Every commit must compile and pass
  `cargo clippy --all-targets --all-features -- -D warnings`.
- **No `unsafe`**, enforced by workspace lints.
- **No `unwrap()`.** Return a `Result` or an `Option`; in tests, use
  `expect("what was expected")` so a failure says what it wanted.
- **Errors**: the CLI uses [`miette`] (it has a terminal and a user to show a
  diagnostic to), the server uses [`anyhow`], and `toboggan-lint` stays
  framework-neutral — it emits serializable diagnostics and the CLI adds the
  miette layer on top.
- **Derive order**: `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default`, then `Serialize, Deserialize` unprefixed, then anything else fully
  qualified (`sqlx::FromRow`, `clap::Parser`, …).
- Prefer turbofish (`parse::<u32>()`) over a type annotation on the binding.

## Tests

```bash
cargo nextest run              # unit and integration tests
cargo test --doc               # doctests — crate READMEs' examples live here
cargo nextest run -p toboggan-lint
```

A lint rule is the ideal unit: build a `RuleContext`, run the rule, and assert on
the `Vec<LintDiagnostic>` that comes back — `toboggan-lint/src/rules/` has one
test module per rule to copy from.

## Commits

Conventional commits, with `!` for a breaking change:

```
feat(lint): tell an author their image will not load
fix(web): stop the deck stealing keys meant for a terminal
feat(action)!: install toboggan from this repo's releases, at a pinned tag
```

Write the subject as what the change does for someone using the project, not as
what you edited. `CHANGELOG.md` is grouped by these types, so a well-scoped
subject line is most of a changelog entry.

## Documentation

Documentation is part of the change, not a follow-up:

- A new flag belongs in the guide deck (`examples/toboggan-guide/slides/`) and in
  the scaffolded `toboggan-cli/templates/toboggan.toml`.

- **The guide's built artifact is checked in.** `toboggan-guide.toml` is
  `include_str!`-embedded into the server, so after editing any slide under
  `examples/toboggan-guide/slides/`, rebuild it:

  ```bash
  cargo run -p toboggan -- build \
    -p examples/toboggan-guide/slides \
    -o examples/toboggan-guide/toboggan-guide.toml
  ```

  CI does this and fails on a diff, so a forgotten rebuild is caught rather than
  shipped.

- Crate README examples are doctests where they can be, so they cannot drift.

## Reporting a bug

A deck that reproduces it is worth more than a description. `toboggan new -p /tmp/repro`, add the slide that breaks, and attach the folder — it is Markdown,
it will be small.

For anything with a security dimension, read [SECURITY.md](SECURITY.md) first.

[`anyhow`]: https://github.com/dtolnay/anyhow
[`miette`]: https://github.com/zkat/miette
