# toboggan-lint

Library-first linter for [Toboggan](https://github.com/ilaborie/toboggan)
presentations. Runs a set of rules over a parsed `Talk` and produces a
framework-neutral, serializable `LintReport`.

It has no CLI/terminal dependencies, so it is reused by `toboggan-cli` (rendered
with miette) and `toboggan-mcp` (serialized to JSON).

```rust
use toboggan_lint::{lint, LintConfig};

let report = lint(&talk, &LintConfig::default());
for diagnostic in &report.diagnostics {
    println!("{:?} [{}] {}", diagnostic.severity, diagnostic.rule.as_str(), diagnostic.message);
}
```

## Rules

`pause/in-part`, `pause/empty-step`, `pause/too-many-steps`, `term/in-part`,
`term/unresolved-cwd`, `term/duplicate-cwd`, `html/nested-step`,
`html/img-missing-alt`, `html/raw-script`, `html/heading-h1`,
`structure/empty-slide`, `structure/title-missing`, `structure/duplicate-part-name`,
`content/excessive-words`, `content/too-many-images`.

The optional `spell` feature adds `spelling/typo` via the `typos` CLI.

Use it from the unified CLI with `toboggan lint <folder>`.
