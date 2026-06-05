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
> The path is resolved from **where you run `toboggan-cli`** (the deck folder),
> not from the slide file. Keep snippets beside your build command.
