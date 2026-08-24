---
name: toboggan-authoring
description: Author Toboggan presentations — folder/markdown conventions, pause/notes/code-embed/terminal directives, styling, and the toboggan CLI/MCP tools. Use when creating or editing a Toboggan slide deck.
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
  _preamble.typ          # optional Typst preamble, replacing the generated one
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
- `<!-- code:rust:snippets/hello.rs -->` embeds an external file as a fenced code
  block, so snippets never drift from the real source. The part after `code:` is
  the fence info string (`rust`, `js`, …). The path is resolved against the deck
  root — the folder *containing* the one you pass to `--path`, so `snippets/`
  sits beside `slides/`, not inside it — and a missing file fails the build.
- `<!-- term: . -->` embeds a live terminal; add the `term-50vh` class to pin its
  pane to half the viewport height.

## Diagrams

A ` ```mermaid ` fence is drawn to SVG while the deck builds — no browser, no
script, and it shows up in the web client, the exported HTML, the PDF and the
thumbnails alike. A diagram that does not parse fails the build.

Tune one from its fence: ` ```mermaid:theme=dark,width=60% `. Parameters are
`theme` (`default`/`dark`/`forest`/`neutral`/`modern`), `background`
(`transparent` by default, or `theme`, or a colour), `width`, `nodeSpacing`,
`rankSpacing`, `aspectRatio`, `maxLabelWidth`, `fastText`, `class`, `alt`.
An unknown parameter fails the build. Deck-wide defaults live in a Mermaid JSON
config named by `[build] mermaid-config` in `toboggan.toml`.

## CLI

- `toboggan -p <folder>` — build + serve with live reload (the default action).
- `toboggan build -p ./slides -o talk.toml` — build to toml/json/yaml/html/typst.
- `toboggan lint -p ./slides` — lint the deck.
- `toboggan pdf -p ./slides` / `toboggan thumbnails -p ./slides` — PDF / overview.
- `--path`/`-p` defaults to the current directory, so from the deck root the
  bare command is enough (`toboggan lint`).

## Quality

One idea per slide, low word counts, alt text on images, `##`/`###` in bodies
(the slide title is the top heading). Run `toboggan lint` before finishing.
