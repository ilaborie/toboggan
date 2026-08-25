+++
title = "Publish to the web"
classes = ["no_title", "wide"]
+++

# Publish to the web

A composite **GitHub Action** builds your deck into single-file HTML, a PDF
handout, and the searchable thumbnail overview — ready for GitHub Pages:

```yaml
- uses: ilaborie/toboggan@v0.2.0
  with:
    folder: ./slides
    outputs: html,pdf,thumbnails
    out-dir: dist
```

<!-- pause -->

It downloads the prebuilt `toboggan` binary for a pinned release, plus `typst`,
then runs the same commands you use locally — so what you preview is what your
audience gets.

> [!NOTE]
> A ready-to-copy consumer workflow lives in `examples/github-pages/pages.yml`.
