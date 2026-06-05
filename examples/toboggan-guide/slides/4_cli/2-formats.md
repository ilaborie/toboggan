+++
title = "Output formats"
classes = ["no_title", "wide"]
+++

# Five output formats

The extension is auto-detected, or force it with `-f`:

```console
$ toboggan-cli ./slides/ -o talk.toml      # default — served by the server
$ toboggan-cli ./slides/ -o talk.json
$ toboggan-cli ./slides/ -o talk.yaml
$ toboggan-cli ./slides/ -o talk.html      # single self-contained file
$ toboggan-cli ./slides/ -f typst -o talk.typ
```

<!-- pause -->

| Format | Use it for |
|---|---|
| `toml` | The artifact `toboggan-server` serves |
| `json` / `yaml` | Pipelines, tooling, inspection |
| `html` | A standalone file you can email or host |
| `typst` | `typst compile talk.typ` → a **PDF** handout |
