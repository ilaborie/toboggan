+++
title = "Lint the deck"
classes = ["no_title", "wide"]
+++

# Lint the deck

`toboggan lint` runs a set of rules over the parsed talk — catching pauses in
section dividers, nested steps, images without `alt`, oversized slides, and more:

```console
$ toboggan lint -p ./slides/
  warning  content/excessive-words   slide 7 — 142 words (max 120)
  error    html/nested-step          slide 12 — nested `.step`
```

<!-- pause -->

| Flag | Purpose |
|---|---|
| `--deny <level>` | Exit non-zero at/above `info`/`warning`/`error` (CI gates) |
| `--json` | Emit a machine-readable `LintReport` |
| `--no-spell` | Skip spell-checking (it runs by default via the `typos` binary) |

> [!TIP]
> The linter is a library (`toboggan-lint`) — the same rules power the MCP
> `lint` tool, so an LLM editing the deck sees exactly what CI sees.
