+++
title = "Author with an LLM"
classes = ["no_title", "wide"]
+++

# Author with an LLM

Two complementary ways to bring an assistant into your deck:

```console
$ toboggan mcp init     # register the MCP authoring server with Claude Code
$ toboggan skills       # install the passive authoring skill
```

<!-- pause -->

The **MCP server** exposes safe, structured tools over your slides folder:

| Tool | What it does |
|---|---|
| `talk_outline` / `stats` / `lint` | Inspect structure and quality |
| `add_part` / `add_slide` | Create sections and slides |
| `set_slide_body` / `set_hidden_in` | Edit content and visibility |
| `reorder` / `move_slide` | Reorganize (renumbers files for you) |

<!-- pause -->

> [!TIP]
> Mutating tools take `dry_run: true` to preview the change set first, and
> front-matter edits preserve your comments — the model edits files the same way
> you would.
