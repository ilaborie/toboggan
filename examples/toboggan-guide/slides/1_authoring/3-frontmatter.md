+++
title = "Frontmatter"
classes = ["no_title", "wide"]
+++

# Per-slide frontmatter

Each slide may open with a TOML block between `+++` delimiters:

```toml
+++
title = "Custom Title"
classes = ["no_title", "wide"]
style = "background: #112; color: gold;"
skip = false
duration = "2m"
hidden_in = ["pdf"]
+++
```

<!-- pause -->

| Field | Type | Effect |
|---|---|---|
| `title` | string | Override the auto-detected slide title |
| `classes` | list | Layout classes (`no_title`, `wide`, `center`, …) |
| `style` | string | Inline CSS on the slide container |
| `skip` | bool | Exclude this slide from the build |
| `duration` | string | Time hint (`30s`, `2m`) for the stats report |
| `hidden_in` | list | Drop from a target: `"web"` or `"pdf"` |

<!-- notes -->
Frontmatter is optional — a bare Markdown file is a perfectly valid slide.
