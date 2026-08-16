+++
title = "Serve it"
classes = ["no_title", "wide"]
+++

# Serve it

While writing, serve the **folder** — built in memory, reloaded on save:

```console
$ toboggan -p ./slides/
# → http://127.0.0.1:8080
```

<!-- pause -->

To serve a deck you already built, point `serve` at the **`.toml`**:

```console
$ toboggan serve -p ./my-talk.toml
$ toboggan serve -p ./my-talk.toml --host 0.0.0.0 --port 9000
```

<!-- pause -->

> [!NOTE]
> `serve` takes a file, the default action takes a folder — that is the whole
> difference. Both are `-p`; neither accepts a bare positional path.

<!-- notes -->
The unified `toboggan` binary is the only one shipped in a release, so these are
the commands a reader will actually have. The old per-crate binaries still build
from source but are not distributed.
