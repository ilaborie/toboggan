+++
title = "One command"
classes = ["no_title", "wide"]
+++

# One command for everything

Point `toboggan` at a folder and it **builds in memory and serves** — with live
reload as you edit. No intermediate file, no second process:

```console
$ toboggan ./slides/
   build + serve  http://localhost:8080
```

<!-- pause -->

The homepage at `/` links every view of the deck:

| Path | What you get |
|---|---|
| `/run` | The live presentation (synced to every client) |
| `/slides` | The thumbnail overview (search + click-to-run) |
| `/guide` | This guide, bundled in |
| `/download.pdf` | A PDF handout, rendered on demand |

> [!TIP]
> Scaffold a fresh deck with `toboggan new my-talk` — it lays out the folder and
> initializes a `jj` repo.
