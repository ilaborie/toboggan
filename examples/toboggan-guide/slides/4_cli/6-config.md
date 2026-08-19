+++
title = "toboggan.toml"
classes = ["no_title", "wide"]
+++

# Configure the deck once

Every flag can live in a `toboggan.toml` beside your slides instead:

```toml
default-command = "serve"   # what a bare `toboggan` does

[build]
theme = "Solarized (dark)"

[serve]
open-presenter = true
```

<!-- pause -->

Files are read from the deck directory, then each parent, then
`~/.config/toboggan/config.toml`. Strongest first:

```
CLI flag  >  TOBOGGAN_*  >  nearest toboggan.toml  >  …  >  default
```

> [!TIP]
> An unknown key is an **error**, not a silent no-op — so a typo tells you.

<!-- notes -->
`toboggan new` writes a file listing every setting, commented out with its
default, so the scaffolded deck is its own reference.

The search order is the useful part: a repo of several talks shares a house
style in a parent directory, and one talk overrides it. The precedence chain
means you can put `open-presenter = true` in the deck and still say
`--port 9000` for a single run without editing anything.

Boolean keys are the one asymmetry — a flag can turn one on, but cannot turn one
back off that the file enabled. Comment it out for a single run.
