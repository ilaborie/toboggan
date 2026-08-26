# Toboggan Examples

Example presentations and the workflows that drive them, all through the single
**`toboggan`** command (`cargo install --path toboggan`, or `cargo run -p toboggan --`
from this repo).

## What's here

| Path | What it is |
|---|---|
| `riir-folder/` | The "Peut-on RIIR de tout ?" talk as a **folder** (one file per slide) |
| `riir-flat.md` | The same talk as a single Markdown file (a content reference) |
| `toboggan-guide/` | The bundled user guide — a full deck that dogfoods every feature |
| `github-pages/pages.yml` | A ready-to-copy workflow that builds + deploys a deck to GitHub Pages |
| `demo-terminal/` | A deck exercising the embedded live-terminal feature |

## The folder layout

`toboggan` builds a **folder** (its input must be a directory): numbered
subfolders are section dividers and numbered `.md` files are slides.

```
riir-folder/
├── _cover.md             # cover slide (title/date in front matter)
├── _head.html            # injected into <head> (fonts, stylesheet)
├── 01-introduction/
│   ├── _part.md          # the section divider
│   └── 01-slide.md       # a slide
└── 02-success-stories/
    ├── _part.md
    └── 01-tools.md
```

> `riir-flat.md` is a single-file *content* reference (slides split by `---`); the
> CLI builds folders, so copy its sections into a folder layout to serve it.

## The everyday loop

```bash
# Build in memory and serve with live reload — open http://localhost:8080
toboggan -p examples/riir-folder

# Scaffold a brand-new deck (lays out the folder + a jj repo)
toboggan new --path my-talk --title "My Talk"
```

The homepage links the live presentation (`/run`), the searchable thumbnail
overview (`/slides`), the bundled guide (`/guide`), and a PDF (`/download.pdf`).

## Build, lint, export

```bash
# Build to a file — the extension picks the format (toml/json/yaml/html/typst)
toboggan build --path examples/riir-folder -o talk.toml
toboggan build --path examples/riir-folder -o talk.html      # single self-contained file

# Lint the deck (CI gate with --deny; --format json for tooling; spell check
# runs by default via `typos`, and --no-spell turns it off)
toboggan lint --path examples/riir-folder

# Export a PDF and a per-slide overview (both need the `typst` binary)
toboggan pdf --path examples/riir-folder
toboggan thumbnails --path examples/riir-folder
```

Serve a prebuilt `.toml` (e.g. the guide artifact) with assets, from the
repository root — the terminal working directories baked into a built deck are
relative to wherever it was built from:

```bash
toboggan serve --public-dir examples/toboggan-guide/public -p examples/toboggan-guide/toboggan-guide.toml
```

## The user guide deck

`toboggan-guide/` is a complete deck that documents Toboggan *using* Toboggan.
Edit `toboggan-guide/slides/`, then rebuild the served artifact — it is
committed, and CI rebuilds it and fails on any diff:

```bash
cd examples/toboggan-guide
mise run build      # toboggan build -p examples/toboggan-guide/slides -o …  (from the repo root, as CI does)
mise run dev        # toboggan -p ./slides/  (build + serve, live reload)
mise run run        # serve the built artifact; also from the repo root
```

`build` and `run` deliberately step up to the repository root, because
`toboggan build` bakes each live terminal's **resolved** working directory into
the artifact, joining the `<!-- term: … -->` value with the directory of the
slide file it appears in. Building from `examples/toboggan-guide` with
`-p ./slides/` produces a file that differs only in those paths — and CI rejects
it. `bacon` (the `toml` job) builds the same artifact and steps up to the root
for the same reason; `dev`, `html` and `pdf` stay local, because their output is
either never written to disk or gitignored and free of baked paths.

The server also bundles this guide at `/guide` on any running deck.

## Author with an LLM

```bash
toboggan mcp init     # register the MCP authoring server with Claude Code
toboggan skills       # install the passive authoring skill
```

The MCP server exposes safe, structured tools over a slides folder
(`talk_outline`, `add_slide`, `set_slide_body`, `reorder`, `move_slide`, …); every
mutating tool supports a `dry_run` preview and preserves your front-matter
comments.

## Publish to the web

`github-pages/pages.yml` is what `toboggan ci` writes into a presentation repo,
committed here so you can read it without running anything. A test keeps the two
identical, so edit the template in `toboggan/src/templates/`, not this copy.

```bash
toboggan ci            # -> .github/workflows/pages.yml, at the repository root
toboggan ci --stdout   # print it instead
```

At its centre is the composite action:

```yaml
- uses: ilaborie/toboggan@v0.2.0
  with:
    folder: ./slides
    outputs: html,pdf,thumbnails
    out-dir: dist
    lint: true
```

The action *lints and builds* — publishing is a separate step, and
`github-pages/pages.yml` hands `out-dir` to `actions/upload-pages-artifact`.
With `lint: true` the diagnostics come back as inline annotations on a pull
request's changed lines, and an error stops the deck reaching the site.
