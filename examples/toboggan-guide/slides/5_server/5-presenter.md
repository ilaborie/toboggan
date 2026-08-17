+++
title = "The presenter view"
classes = ["no_title", "wide"]
+++

# What you see, what the room sees

`/presenter` is the second window: the deck goes on the projector, this one
stays on your screen.

<!-- pause -->

- The **current** slide, and the **next** one, so you can see what is coming
- The **speaker notes** for where you are — everything after `<!-- notes -->`
- A **clock**, an **elapsed timer** (click it to restart), the slide and reveal
  counters, and how far ahead or behind your `duration` front matter you are

<!-- pause -->

```console
$ toboggan -p ./slides/ --open-presenter
```

> [!TIP]
> It is the same application as `/run`, on the same socket — the arrow keys
> drive the deck from either window. Neither one is in charge.

<!-- notes -->
The two previews never start a slide's embedded terminals. The shell belongs to
the deck the room is watching; a second one here would be a different session
showing different output.

The pacing readout only appears when the deck declares durations — without a
plan there is nothing to be late for.
