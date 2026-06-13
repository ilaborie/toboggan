+++
title = "Output formats"
classes = ["no_title", "wide"]
+++

# Five output formats

The extension is auto-detected, or force it with `-f`:

```console
$ toboggan build ./slides/ -o talk.toml      # default — served by the server
$ toboggan build ./slides/ -o talk.json
$ toboggan build ./slides/ -o talk.yaml
$ toboggan build ./slides/ -o talk.html      # single self-contained file
$ toboggan build ./slides/ -f typst -o talk.typ
```

<!-- pause -->

| Format | Use it for |
|---|---|
| `toml` | The artifact `toboggan serve` reads |
| `json` / `yaml` | Pipelines, tooling, inspection |
| `html` | A standalone file you can email or host |
| `typst` | `typst compile talk.typ` → a **PDF** handout |
