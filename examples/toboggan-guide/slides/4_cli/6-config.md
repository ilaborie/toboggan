+++
title = "toboggan.toml"
classes = ["no_title", "wide"]
+++

# Configure the deck once

Every flag can live in a `toboggan.toml` beside your slides instead:

```toml
default-command = "serve"   # what a bare `toboggan` does

[build]
theme = "Monokai"
wpm = 130

[serve]
open-presenter = true

[lint]
max-duration = "45m"
```

<!-- pause -->

Files are read from the deck directory, then each parent, then
`~/.config/toboggan/config.toml` — so a repo of several talks shares a house
style and one talk overrides it. Strongest first:

```
CLI flag  >  TOBOGGAN_*  >  nearest toboggan.toml  >  …  >  user global  >  default
```

> [!TIP]
> An unknown key is an **error**, not a silent no-op — so a typo tells you
> instead of quietly doing nothing. `toboggan new` writes a file listing every
> setting, commented out with its default.

<!-- notes -->
The precedence order is the useful part: you can put `open-presenter = true` in
the deck and still say `--port 9000` for one run without editing anything.

Boolean keys are the one asymmetry — a flag can turn something on, but cannot
turn it back off if the file enabled it. Comment it out for a single run.
