# Toboggan Examples

This directory contains examples of different ways to create presentations with Toboggan.

## RIIR Talk Examples

The "Peut-on RIIR de tout ?" (Can we RIIR everything?) talk is provided in two different formats to demonstrate the flexibility of the Toboggan CLI:

### Flat File Format: `riir-flat.md`

A single Markdown file containing the entire presentation. This format is ideal for:
- Simple presentations
- Quick prototyping
- Version control (single file to track)
- Easy sharing

**Structure:**
- `# Title` - Talk title
- `> Notes` - Speaker notes for the cover slide
- `---` - Slide separators
- `## Heading` - Part slides (section dividers)
- `### Heading` - Regular slide titles

**Usage:**
```bash
# Basic usage
cargo run --package toboggan-cli -- examples/riir-flat.md -o examples/riir-flat-output.toml

# With custom date
cargo run --package toboggan-cli -- examples/riir-flat.md --date 2024-12-25 -o examples/riir-flat-output.toml
```

### Folder-Based Format: `riir-folder/`

A directory structure where each folder represents a section and each file represents a slide. This format is ideal for:
- Complex presentations
- Team collaboration
- Modular content management
- Rich media integration

**Structure:**
```
riir-folder/
├── title.md              # Talk title (or use folder name)
├── _cover.md             # Cover slide content
├── 01-introduction/      # Section folder
│   ├── _part.md         # Part slide for this section
│   └── 01-slide.md      # Regular slides
├── 02-success-stories/   # Another section
│   ├── _part.md
│   ├── 01-tools.md
│   └── 02-reasons.md
└── ...
```

**Special Files:**
- `title.md` / `title.txt` - Talk title (fallback to folder name)
- `_cover.md` - Cover slide (special styling)
- `_part.md` - Part slide within a folder (section divider)
- `*.md` / `*.html` - Regular slides (sorted by filename)

**Usage:**
```bash
# Basic usage (uses today's date)
cargo run --package toboggan-cli -- examples/riir-folder -o examples/riir-folder-output.toml

# With custom date
cargo run --package toboggan-cli -- examples/riir-folder --date 2024-12-25 -o examples/riir-folder-output.toml
```

## Generated Outputs

Both approaches generate equivalent TOML files that can be served by the Toboggan server:

- `riir-flat-output.toml` - Generated from the flat file
- `riir-folder-output-fixed.toml` - Generated from the folder structure

## Folder-Based Features

The folder-based approach provides additional capabilities:

### 1. **Hierarchical Organization**
- Folders automatically become Part slides
- Contents are processed in alphabetical order
- Clear separation of concerns

### 2. **Flexible Content Types**
- `.md` files - Markdown content (converted to HTML)
- `.html` files - Raw HTML content
- Mixed content types in the same presentation

### 3. **Special File Handling**
- `_cover.md` - Creates a Cover slide
- `_part.md` - Customizes the Part slide for a folder
- Date management via `--date` CLI argument

### 4. **Team Collaboration**
- Different team members can work on different sections
- Easy to reorganize content by renaming folders/files
- Git-friendly structure with granular change tracking

## Converting Between Formats

You can use the CLI to convert between formats:

1. **Markdown to TOML**: Direct conversion for serving
2. **Folder to TOML**: Structured conversion with automatic organization
3. **Manual conversion**: Extract sections from flat file into folders for better organization

## Best Practices

### Flat File Format
- Use clear section breaks with `---`
- Keep speaker notes in blockquotes `>`
- Use heading levels consistently (H2 for parts, H3 for slides)

### Folder-Based Format
- Use numbered prefixes for ordering (01-, 02-, etc.)
- Keep folder names descriptive but concise
- Place shared resources in a dedicated folder
- Use consistent naming conventions across the team

Both formats support the full range of Toboggan features including HTML content, speaker notes, and different slide types.