+++
title = "term — live terminals"
classes = ["no_title", "wide", "term-50vh"]
+++

# Live terminals

Embed a real PTY in a slide: `<!-- term: . -->`, plus `:light` for the theme
and `| cmd` to run something on connect.

The terminal below is **live** — click it and type:

<!-- term: . -->

<!-- notes -->
The full syntax: `<!-- term: ./src :light -->`, `<!-- term: . | bacon test -->`.

Add the `term-50vh` class to pin the terminal pane to half the viewport height
(this slide uses it) — otherwise it fills the space left below the content.

Three custom properties reach inside a terminal's shadow root, where selectors
cannot: `--terminal-radius` rounds its corners, `--terminal-titlebar-display`
can drop the chrome, and `--terminal-font-size` opens it at another size — a
plain number of CSS pixels, clamped to 8-32, the same range `Cmd +` / `Cmd -`
covers on stage. Declare them on the `.toboggan-terminal` element or anywhere
above it.

The chrome takes its colours the same way: `--terminal-window-bg`,
`--terminal-border`, `--terminal-shadow`, `--terminal-titlebar-bg`,
`--terminal-titlebar-border`, `--terminal-title-color`,
`--terminal-focus-ring`, and `--terminal-btn-close` / `-minimize` / `-maximize`
for the three buttons. Undeclared, each keeps its Catppuccin value.

The palette inside the canvas is one property per slot, hex only:
`--terminal-foreground`, `--terminal-background`, `--terminal-cursor`,
`--terminal-selection-foreground`, `--terminal-selection-background`, and the
sixteen ANSI colours from `--terminal-black` to `--terminal-bright-white`. The
window background follows the canvas, so the frame matches it without saying
it twice.

The font size and the palette are read when the terminal opens, because
rioterm fixes both when it builds the renderer, and once more when it is first
laid out. Declare them in the slide's own `<style>`: a stylesheet reached
through `@import` that lands late costs a rebuild of the terminal. The chrome
is plain CSS and has no such race.

Multiple `term` comments on one slide render side by side. The shell is chosen
by the server's --shell flag. The fenced examples above stay as text; the bare
one below is a working terminal. `term-50vh` fixes the pane at 50vh so it does
not push past the slide; without it the terminal flexes to fill the remaining
height.
