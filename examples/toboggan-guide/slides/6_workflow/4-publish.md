+++
title = "Publish to the web"
classes = ["no_title", "wide"]
+++

# Publish to the web

`toboggan ci` writes the whole GitHub Pages workflow — single-file HTML, a PDF
handout, and the searchable thumbnail overview:

```bash
toboggan ci                    # -> .github/workflows/pages.yml
toboggan new -p my-talk --ci   # or scaffold with it in place
```

<!-- pause -->

At its centre is a composite action that downloads the prebuilt binary for a
pinned release, plus `typst`, then runs the commands you use locally:

```yaml
- uses: ilaborie/toboggan@v0.2.0
  with:
    folder: ./slides
    outputs: html,pdf,thumbnails
    lint: true
```

<!-- notes -->
The pin matches the binary that generated the file: `toboggan ci` reads its own
`CARGO_PKG_VERSION`, and a test fails the build if any doc falls behind it.

`lint: true` keeps a deck with an error in it from ever reaching the site — the
diagnostics come back as inline annotations on the pull request's changed lines.
