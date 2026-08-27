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
- A **clock**, an **elapsed timer** you can pause and reset, the slide and
  reveal counters, and how far ahead or behind your `duration` front matter you are

<!-- pause -->

```console
$ toboggan -p ./slides/ --open-presenter
```

> [!TIP]
> Both panes *are* `/run`, framed — same stylesheet, same viewport rules — so
> what you see is what the room sees. Arrow keys drive the deck from either
> window, and so do the `‹` `›` buttons. Neither one is in charge.

<!-- notes -->
The panes are iframes of the deck, laid out against a fixed 1280x720 viewport
and scaled to fit — which is what makes them break their lines where the
projector does, and what keeps a deck's own `_head.html` out of this view's
styling.

They never start a slide's embedded terminals. The shell belongs to the deck the
room is watching; a second one here would be a different session showing
different output.

The pacing readout only appears when the deck declares durations — without a
plan there is nothing to be late for.
