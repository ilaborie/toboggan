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

> [!TIP] `toboggan pdf` names every slide that spilled onto a second page. The Typst preamble is generated, unless the deck ships a `slides/_preamble.typ` — which *replaces* it, and then owns everything it set up.
