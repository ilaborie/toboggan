+++
title = "Keyboard"
classes = ["no_title", "wide"]
quake_cwd = "."
+++

# Driving the deck

| Key | Does |
|---|---|
| `→` `←` | Next / previous slide |
| `↓` `↑` `Space` `PageDown` `PageUp` `Backspace` | Next / previous **step** |
| `Home` `End` | First / last slide |
| digits then `⏎` | Go to that slide number |
| `f` | Fullscreen |
| `.` `w` | Blank the screen, black or white |
| `b` | Blink — flash every other client |
| `` ` `` | The quake terminal |
| `F1` | This list, in the browser |
| `g` `/` `Ctrl`/`Cmd`+`K` | The slide picker (`/presenter`) |

<!-- pause -->

> [!TIP]
> A clicker sends `PageUp`/`PageDown`, bound to **steps** rather than slides on
> purpose — so the remote walks the whole deck instead of skipping every build.

<!-- notes -->
`.` and `w` are for the moment someone asks a question and you want the room
looking at you instead of at the slide. They are handled in the tab, not on the
server, so blanking your screen does not blank anyone else's.

The `F1` dialog is generated from the keymap itself, so it cannot fall out of
date with what the keys actually do. The picker's three keys are not in it: they
belong to `/presenter` alone, and a shared dialog naming them would promise the
deck something it cannot do. The `▦` button in the status strip names all three
in its tooltip, which is the one place they can be read *before* the picker is
open.

The quake terminal opens in `quake_cwd` — set per slide, as this one does, or
once for the whole deck in the cover's front matter. It stays open and keeps its
shell as you walk the deck; only a slide that names a *different* directory
restarts the session, so a build running in it survives the talk.
