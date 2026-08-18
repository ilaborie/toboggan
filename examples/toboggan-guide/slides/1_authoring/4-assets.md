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
> The leading `/public/` is not optional. Slides render at `/run`, and only
> `/public/` is mapped to a directory — so `![…](diagram.webp)` asks for
> `/diagram.webp`, which nothing serves. The `link/broken` lint rule catches
> exactly this, and says where the file actually is.

`toboggan build -o deck.html` copies `public/` next to the exported file, so a
deck stays self-contained when you publish it.

<!-- notes -->
This is the single most common way a deck breaks between the laptop and the
projector: the image is right there on disk, the URL is one segment off, and the
slide renders with a broken image icon in front of the room.

`--public-dir` overrides the location if the folder is somewhere else.
