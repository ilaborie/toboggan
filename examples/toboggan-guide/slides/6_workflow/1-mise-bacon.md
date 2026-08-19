+++
title = "The dev loop"
classes = ["no_title", "wide"]
+++

# The edit loop

One terminal. The default action builds the folder **in memory** and watches it,
so there is no `.toml` to regenerate and no second process to run:

```console
$ toboggan -p ./slides/
   build + serve  http://localhost:8080
```

Save a slide → rebuild → the browser reloads.

<!-- pause -->

Wrap the rest in `mise` tasks:

```console
$ mise run dev         # the loop above
$ mise run build       # slides/ → toboggan-guide.toml
$ mise run pdf         # typst export → PDF
$ mise run run         # serve the built .toml with its public/ assets
```

> [!TIP]
> You only need a built `.toml` to *ship* a deck. While writing, skip it.

<!-- notes -->
See this deck's own `mise.toml` for the task definitions.

This used to be a two-terminal loop: bacon rebuilding the .toml on save, plus a
server with --watch reloading it. The in-memory default action collapsed both
into one command, so the deck's bacon.toml is now only there for regenerating
the committed artifact.
