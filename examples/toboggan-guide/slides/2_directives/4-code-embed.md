+++
title = "code — embed a file"
classes = ["no_title", "wide"]
+++

# Embed a source file

Pull an external file in as a fenced code block — no copy-paste drift:

```markdown
<!-- code:rust:snippets/hello.rs -->
```

<!-- pause -->

…which renders, live from disk, as:

<!-- code:rust:snippets/hello.rs -->

> [!IMPORTANT]
> The path resolves against the **deck root** — the folder *containing* the one
> you pass to `-p`, never the slide file and never your shell's current
> directory. So `snippets/` sits **beside** `slides/`, not inside it, and the
> same embed resolves identically wherever you run the build from.

Absolute paths and `..` are refused: an embed cannot reach outside the deck.
