+++
title = "Serve it"
classes = ["no_title", "wide"]
+++

# Serve it

```console
$ toboggan-server ./my-talk.toml
# → http://127.0.0.1:8080
```

<!-- pause -->

Bind elsewhere for the room to join from their laptops:

```console
$ toboggan-server --host 0.0.0.0 --port 9000 ./my-talk.toml
```

<!-- pause -->

> [!NOTE]
> The server takes the compiled **`.toml`** (not the slides folder). Build
> first with `toboggan-cli`, then serve — or let `--watch` rebuild for you.
