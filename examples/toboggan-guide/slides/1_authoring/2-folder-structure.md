+++
title = "Folder structure"
classes = ["no_title", "wide"]
+++

# A deck is a folder

`toboggan` reads a **directory** of Markdown/HTML files. Order comes from filenames; sections come from sub-folders.

```text
my-talk/
├── _head.html          # injected into <head> (fonts, stylesheet)
├── _footer.html        # footer component (progress bar, etc.)
├── _cover.md           # the cover / title slide
├── 1_intro/
│   ├── _part.md        # section divider slide
│   ├── 1-hello.md
│   └── 2-agenda.md
└── 2_deep-dive/
    ├── _part.md
    └── 1-details.md
```

<!-- pause -->

> [!NOTE] Only these names are special (exact match): `_cover.md`, `_part.md`, `_head.html`, `_footer.html`. Everything else is a slide, sorted alphabetically — so prefix files with `1-`, `2-`, … to control order.
