# toboggan-mcp

An [MCP](https://modelcontextprotocol.io) server (built on `rmcp`) exposing
[Toboggan](https://github.com/ilaborie/toboggan) authoring tools over stdio, so
an LLM client can inspect and edit a presentation folder.

## Tools

Inspection:

- `talk_outline` — the cover, parts, and slides as they exist on disk, each with
  the relative `path` the editing tools address, plus titles, `hidden_in`, and
  `skip` flags.
- `stats` — slide counts and total word count.
- `lint` — the full lint report (from `toboggan-lint`).
- `advice` — embedded authoring guidance.

Editing (each returns a `ChangeSet`; mutating tools accept `dry_run` to preview):

- `add_part` / `add_slide` — create a numbered section or slide.
- `new_presentation` — scaffold a complete new deck folder at a subpath.
- `set_slide_title` / `set_part_title` / `set_slide_body` — edit content.
- `set_hidden_in` — set the render targets a slide is hidden in (`web`/`pdf`).
- `skip_slide` — toggle the `skip` flag.
- `remove_slide` / `remove_part` — delete a slide or section.
- `reorder` — renumber a section's slides (or top-level parts/slides).
- `move_slide` — move a slide to another section/top level at a position.

Mutations go through a safe `Workspace`: paths are confined to the presentation
root, writes are atomic, new files are numbered deterministically, and
front-matter edits use `toml_edit` so comments and unknown keys survive.

Tools assume **sequential** calls (one tool result awaited before the next) — the
intended LLM-client usage. Issuing concurrent mutations can race on numbering.

## Usage

```bash
toboggan mcp -p ./my-talk          # serve over stdio (defaults to the cwd)
toboggan mcp serve -p ./my-talk    # the same thing, named explicitly
toboggan mcp init                  # register with Claude Code (claude mcp add)
```

`-p/--path` is the presentation directory. Point it at the deck root and the
workspace re-anchors itself on the `slides/` subfolder, so both `./my-talk` and
`./my-talk/slides` do the right thing. `toboggan new` writes a project-local
`.mcp.json` pointing here, so a scaffolded deck is wired up already.
