# Authoring a Toboggan presentation

## Folder layout

A presentation is a `slides/` folder:

- `_cover.md` — the cover slide (front matter `title`, `date`).
- `NN-section/` — a numbered section folder, containing:
  - `_part.md` — the section divider slide.
  - `NN-slide.md` — content slides (sorted by filename).
- `_head.html` / `_footer.html` — optional custom head/footer.
- assets live in a sibling `public/` folder.

## Front matter

Each `.md` file may start with a TOML front-matter block:

```
+++
title = "Slide title"
classes = ["no_title"]   # hide the rendered title (use a body `#` heading instead)
skip = true              # hide this slide
hidden_in = ["pdf"]      # exclude from a render target (web | pdf)
disabled_rules = ["html/img-missing-alt"]  # silence lint rules for this slide
+++
```

## Directives (HTML comments in the body)

- `<!-- pause -->` — split the slide into reveal steps. Never use on cover/part slides.
- `<!-- pause :class -->` — add CSS classes to a step.
- `<!-- notes -->` — everything after is speaker notes.
- `<!-- lint-disable rule-id … -->` — silence lint rules for this slide.
- terminals — see the guide for the `terminal` directive.

## Quality tips

- One idea per slide; keep word counts low.
- Always give images alt text.
- Use `##`/`###` in bodies; the slide title is the top heading.
- Run `lint` to catch bad HTML, misused `pause`/terminals, and structure issues.
