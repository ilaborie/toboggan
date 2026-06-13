+++
title = "Convert a folder"
classes = ["no_title", "wide"]
+++

# Convert a folder

```console
$ toboggan-cli ./slides/ -o my-talk.toml
✅ Successfully wrote 12 slides to my-talk.toml
```

<!-- pause -->

Override metadata without touching the files:

```console
$ toboggan-cli ./slides/ \
    --title "My Conference Talk" \
    --date 2026-09-15 \
    -o my-talk.toml
```

<!-- pause -->

| Flag | Purpose |
|---|---|
| `-o, --output` | Output file (extension picks the format) |
| `-t, --title` | Override the title |
| `-d, --date` | Override the date (`YYYY-MM-DD`) |

> [!NOTE]
> The input must be a **folder** — a single flat `.md` file is not accepted.
