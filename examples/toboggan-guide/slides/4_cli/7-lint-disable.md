+++
title = "Silencing a rule"
classes = ["no_title", "wide"]
+++

# When the rule is wrong

Sometimes a slide is meant to be dense. Silence a rule for that one slide:

```markdown
<!-- lint-disable content/excessive-words -->
```

<!-- pause -->

Or in its front matter, which is easier to see when you come back to it:

```toml
+++
title = "The whole API"
disabled_rules = ["content/excessive-words"]
+++
```

> [!TIP]
> Deck-wide, use `[lint] disabled`. Better still, lower the severity with
> `[lint.severity]`: an `info` still tells you the slide is dense, it just
> stops failing the build.

<!-- notes -->
An unknown rule id is reported rather than ignored, so a renamed rule cannot
silently stop applying.

Suppression is per slide, not per line: diagnostics are not line-tracked, so a
directive covers the whole slide it appears on.

Every diagnostic prints the id you would use here, so you never have to go
looking for the name.
