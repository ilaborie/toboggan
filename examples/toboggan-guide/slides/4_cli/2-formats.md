+++
title = "Output formats"
classes = ["no_title", "wide"]
+++

# Five output formats

The extension is auto-detected, or force it with `-f`:

```console
$ toboggan build -p ./slides/ -o talk.toml      # default — served by the server
$ toboggan build -p ./slides/ -o talk.json
$ toboggan build -p ./slides/ -o talk.yaml
$ toboggan build -p ./slides/ -o talk.html      # single self-contained file
$ toboggan build -p ./slides/ -f typst -o talk.typ
```

<!-- pause -->

| Format | Use it for |
|---|---|
| `toml` | The artifact `toboggan serve` reads |
| `json` / `yaml` | Pipelines, tooling, inspection |
| `html` | A standalone file you can email or host |
| `typst` | `typst compile talk.typ` → a **PDF** handout |

<!-- pause -->

> [!TIP] The Typst preamble — theme, aspect ratio, text size, margins — is generated, unless the deck has a `slides/_preamble.typ` (or you pass `--typst-preamble <FILE>`). Yours *replaces* it, so it owns the imports the slides need: touying, codly, codly-languages, gentle-clues, mitex.
