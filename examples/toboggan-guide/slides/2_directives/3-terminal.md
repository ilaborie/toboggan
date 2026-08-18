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

Multiple `term` comments on one slide render side by side. The shell is chosen
by the server's --shell flag. The fenced examples above stay as text; the bare
one below is a working terminal. `term-50vh` fixes the pane at 50vh so it does
not push past the slide; without it the terminal flexes to fill the remaining
height.
