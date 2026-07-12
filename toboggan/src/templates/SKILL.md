---
name: toboggan-authoring
description: Author Toboggan presentations — folder/markdown conventions, pause/notes/terminal directives, styling, and the toboggan CLI/MCP tools. Use when creating or editing a Toboggan slide deck.
---

# Authoring Toboggan presentations

Toboggan builds a slide deck from a `slides/` folder of Markdown files.

## When the toboggan MCP server is connected

Prefer its tools over editing files by hand:

- `talk_outline` — inspect parts/slides and their indices.
- `stats` / `lint` — check word counts and catch issues.
- `add_part` / `add_slide` — create sections and slides safely.
- `advice` — authoring guidance.

If MCP is not connected, edit the folder directly following the conventions below.

## Folder layout

```
slides/
  _cover.md              # cover slide (front matter: title, date)
  01-introduction/       # numbered section folder
    _part.md             # section divider slide
    01-welcome.md        # content slide (sorted by filename)
  _head.html             # optional custom <head>
public/                  # assets (sibling of slides/)
```

## Front matter (TOML between `+++`)

```
+++
title = "Slide title"
classes = ["no_title"]   # hide the rendered title; use a body `#` heading instead
skip = true              # hide this slide
hidden_in = ["pdf"]      # exclude from a render target (web | pdf)
+++
```

## Directives (HTML comments)

- `<!-- pause -->` splits a slide into reveal steps. Never on cover/part slides.
- `<!-- pause :class -->` adds CSS classes to a step.
- `<!-- notes -->` — everything after is speaker notes.
- `<!-- term: . -->` embeds a live terminal; add the `term-50vh` class to pin its
  pane to half the viewport height.

## CLI

- `toboggan <folder>` — build + serve with live reload (the default action).
- `toboggan build ./slides -o talk.toml` — build to toml/json/yaml/html/typst.
- `toboggan lint ./slides` — lint the deck.
- `toboggan pdf ./slides` / `toboggan thumbnails ./slides` — PDF / overview.

## Quality

One idea per slide, low word counts, alt text on images, `##`/`###` in bodies
(the slide title is the top heading). Run `toboggan lint` before finishing.
