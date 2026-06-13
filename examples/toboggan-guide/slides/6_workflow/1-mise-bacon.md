+++
title = "The dev loop"
classes = ["no_title", "wide"]
+++

# The edit loop

Two terminals, instant feedback:

```console
# 1 — rebuild the .toml whenever a slide changes
$ bacon                # uses bacon.toml in the deck folder

# 2 — serve with live reload
$ toboggan-server --public-dir ./public/ --watch ./my-talk.toml
```

<!-- pause -->

Or wrap both in `mise` tasks (see this deck's `mise.toml`):

```console
$ mise run build       # slides/ → my-talk.toml
$ mise run dev          # serve + --watch
$ mise run pdf          # typst export → PDF
```

> [!TIP]
> Save a slide → `bacon` regenerates the `.toml` → `--watch` reloads the
> browser. No manual rebuild, no refresh.
