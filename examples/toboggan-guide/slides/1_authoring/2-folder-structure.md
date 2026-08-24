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
├── _preamble.typ       # optional: replaces the generated Typst preamble
├── _cover.md           # the cover / title slide
├── 1_intro/
│   ├── _part.md        # section divider slide
│   ├── 1-hello.md
│   └── 2-agenda.md
└── 2_deep-dive/        # same shape as 1_intro
```

<!-- pause -->

> [!NOTE] Only `_cover.md`, `_part.md`, `_head.html`, `_footer.html` and `_preamble.typ` are special (exact match). Everything else is a slide, sorted by filename.
