+++
classes = ["no_title"]
+++

# Frontmatter Reference

Each slide supports TOML frontmatter between `+++` delimiters:

```toml
+++
title = "Custom Title"
classes = ["no_title", "wide"]
style = "background: linear-gradient(#1a1a2e, #16213e);"
skip = false
duration = "2m"
date = "2026-04-04"
hidden_in = ["pdf"]
+++
```

<!-- pause -->

| Field | Type | Description |
|---|---|---|
| `title` | string | Override auto-detected title |
| `classes` | list | CSS classes for layout |
| `style` | string | Inline CSS rules |
| `skip` | bool | Skip this slide |
| `duration` | string | Time hint (humantime) |
| `date` | string | Slide date |
| `hidden_in` | list | Exclude from targets: `"web"`, `"pdf"` |
