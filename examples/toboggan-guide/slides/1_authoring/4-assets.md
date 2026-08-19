+++
title = "Images and assets"
classes = ["no_title", "wide"]
+++

# Images and assets

Anything that is not a slide lives in `public/`, a sibling of `slides/`:

```
my-talk/
├── public/
│   ├── diagram.webp
│   └── style.css
└── slides/
```

<!-- pause -->

It is served at `/public/`, so that is how a slide asks for it:

```markdown
![Architecture](/public/diagram.webp)
```

> [!WARNING]
> The `public/` segment is not optional — `![…](diagram.webp)` asks for
> `/diagram.webp`, which nothing serves. The `link/broken` rule catches it.
> The leading slash *is* optional: `/public/x`, `public/x` and `./public/x`
> all resolve, which is why `_head.html` can write `./public/style.css`.

<!-- notes -->
Slides render at `/run`, and only `/public/` is mapped to a directory on disk,
which is why the prefix matters.

This is the single most common way a deck breaks between the laptop and the
projector: the image is right there on disk, the URL is one segment off, and the
slide renders with a broken image icon in front of the room.

`toboggan build -o deck.html` copies `public/` next to the exported file, so a
deck stays self-contained when you publish it. When *serving*, `--public-dir`
overrides which folder is mounted at `/public` — it is a serve flag, not a
build one.
