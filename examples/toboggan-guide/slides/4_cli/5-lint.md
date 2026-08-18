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
| `--format human` | Coloured lines for a terminal (the default) |
| `--format github` | Workflow commands, which become inline PR annotations |
| `--format sarif` | SARIF 2.1.0, for GitHub code scanning |
| `--format json` | The `LintReport` as JSON (`--json` is a shorthand) |
| `--no-spell` | Skip spell-checking (it runs by default via the `typos` binary) |

> [!TIP]
> Diagnostics carry the file they came from, so `--format github` annotates the
> offending slide in a pull request. The linter is a library (`toboggan-lint`) —
> the same rules power the MCP `lint` tool, so an LLM editing the deck sees
> exactly what CI sees.
