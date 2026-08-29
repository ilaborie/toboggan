+++
title = "The presenter view"
classes = ["no_title", "wide"]
+++

# What you see, what the room sees

`/presenter` is the second window: the deck goes on the projector, this one
stays on your screen.

<!-- pause -->

- The **current** slide, and a still of the **next** one, so you can see what is
  coming
- The **speaker notes** for where you are — everything after `<!-- notes -->`
- A **clock**, an **elapsed timer** you can pause and reset, the slide and
  reveal counters, and how far ahead or behind your `duration` front matter you are
- `g`, `/` or `Ctrl`/`Cmd`+`K` opens the **slide picker** — every slide, searchable

<!-- pause -->

```console
$ toboggan -p ./slides/ --open-presenter
```

> [!TIP]
> The big pane *is* `/run`, framed — same stylesheet, same viewport rules — so
> what you see is what the room sees. Arrow keys drive the deck from either
> window, and so do the `‹` `›` buttons. Neither one is in charge.

<!-- notes -->
The current pane is an iframe of the deck, laid out against a fixed 1280x720
viewport and scaled to fit — which is what makes it break its lines where the
projector does, and what keeps a deck's own `_head.html` out of this view's
styling. It never starts a slide's embedded terminals: the shell belongs to the
deck the room is watching, and a second one here would be a different session
showing different output.

The next slide is a photograph rather than a second deck — the same still the
picker shows. Forty small stills is forty pictures; forty iframes would be forty
copies of the deck.

The picker searches each slide's title, its part, its body and *these notes* —
and the words inside its diagrams — which is usually how you remember a slide
mid-talk. Arrows move, `⏎` jumps, `Esc` closes.

The pacing readout only appears when the deck declares durations — without a
plan there is nothing to be late for.
