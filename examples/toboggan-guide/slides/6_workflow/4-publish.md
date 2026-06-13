+++
title = "Publish to the web"
classes = ["no_title", "wide"]
+++

# Publish to the web

A composite **GitHub Action** builds your deck and ships it to GitHub Pages —
single-file HTML, a PDF handout, and the searchable thumbnail overview:

```yaml
- uses: ilaborie/toboggan@v1
  with:
    folder: ./slides
    outputs: html,pdf,thumbnails
    deploy-pages: true
```

<!-- pause -->

It installs `toboggan` (prebuilt binary, falling back to `cargo install`) and
`typst`, then runs the same commands you use locally — so what you preview is
what your audience gets.

> [!NOTE]
> A ready-to-copy consumer workflow lives in `examples/github-pages/pages.yml`.
