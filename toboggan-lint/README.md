# toboggan-lint

Library-first linter for [Toboggan](https://github.com/ilaborie/toboggan)
presentations. Runs a set of rules over a parsed `Talk` and produces a
framework-neutral, serializable `LintReport`.

It has no CLI/terminal dependencies, so it is consumed by the `toboggan` binary (printed
with `owo_colors`) and `toboggan-mcp` (serialized to JSON).

```rust
use toboggan_lint::{lint, LintConfig};

let report = lint(&talk, &LintConfig::default());
for diagnostic in &report.diagnostics {
    println!("{:?} [{}] {}", diagnostic.severity, diagnostic.rule.as_str(), diagnostic.message);
}
```

## Rules

`pause/in-part` (cover and part slides), `pause/empty-step`, `pause/too-many-steps`,
`term/in-part`, `term/unresolved-cwd`, `term/duplicate-cwd`, `html/nested-step`,
`html/img-missing-alt`, `html/raw-script`, `html/heading-h1`,
`structure/empty-slide`, `structure/title-missing`, `structure/duplicate-part-name`,
`content/excessive-words`, `content/too-many-images`.

With the optional `spell` feature, `spelling/typo` runs as part of the default
suite via the `typos` CLI. It degrades silently when `typos` is not installed.

## Disabling rules per slide

Diagnostics are per-slide, so a rule can be silenced for a whole slide via front
matter or a body directive (per-line disabling is not supported):

```md
+++
disabled_rules = ["html/img-missing-alt"]
+++

<!-- lint-disable html/raw-script pause/empty-step -->
```

Use it from the unified CLI with `toboggan lint <folder>` (`--no-spell` opts out
of spell checking).
