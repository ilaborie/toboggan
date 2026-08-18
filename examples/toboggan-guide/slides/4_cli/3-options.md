+++
title = "Handy options"
classes = ["no_title", "wide"]
+++

# Handy options

```console
$ toboggan build -p ./slides/ --theme "Solarized (dark)" -o talk.toml
$ toboggan build -p ./slides/ --no-counter   # don't auto-number parts/slides
$ toboggan build -p ./slides/ --wpm 130      # tune the duration estimate
```

<!-- pause -->

Every build prints a **stats report** — word counts, per-part breakdown, and
duration scenarios for slow/normal/fast speakers:

```text
 Part             Slides   Words   Percentage   Duration
 1. Authoring          3     210        18.4%       1:24
 2. Directives         5     330        28.9%       2:12
 ...
```

> [!TIP]
> Add `--exclude-notes-from-duration` to time only what the audience sees.
