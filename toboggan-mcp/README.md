# toboggan-mcp

An [MCP](https://modelcontextprotocol.io) server (built on `rmcp`) exposing
[Toboggan](https://github.com/ilaborie/toboggan) authoring tools over stdio, so
an LLM client can inspect and edit a presentation folder.

## Tools

- `talk_outline` — list parts/slides with indices, kinds, titles, hidden flags.
- `stats` — slide counts and total word count.
- `lint` — the full lint report (from `toboggan-lint`).
- `add_part` — create a numbered section folder with a `_part.md`.
- `add_slide` — create a slide (top level or inside a section).
- `advice` — embedded authoring guidance.

Mutations go through a safe `Workspace`: paths are confined to the presentation
root, writes are atomic, and new files are numbered deterministically.

## Usage

```bash
toboggan mcp --dir ./slides        # serve over stdio
toboggan mcp init                  # register with Claude Code (claude mcp add)
```
