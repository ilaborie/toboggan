# Toboggan CLI

Convert Markdown presentations to Toboggan format with advanced features for speaker notes, progressive reveals, and presentation statistics.

[![Crates.io](https://img.shields.io/crates/v/toboggan-cli.svg)](https://crates.io/crates/toboggan-cli)
[![Documentation](https://docs.rs/toboggan-cli/badge.svg)](https://docs.rs/toboggan-cli)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/ilaborie/toboggan)

## Installation

```bash
# From source
cargo install --path toboggan-cli

# From crates.io (when published)
cargo install toboggan-cli
```

## Quick Start

```bash
# Convert a presentation folder to TOML
toboggan-cli slides/ -o presentation.toml

# Override title and date
toboggan-cli slides/ --title "My Talk" --date "2025-03-15" -o talk.toml

# Use different output format
toboggan-cli slides/ -f json -o presentation.json
```

## CLI Options

```
toboggan-cli [OPTIONS] <INPUT>

Arguments:
  <INPUT>                    Input folder containing presentation files

Options:
  -o, --output <FILE>        Output file (default: stdout)
  -t, --title <TITLE>        Override presentation title
  -d, --date <DATE>          Override date (YYYY-MM-DD format)
  -f, --format <FORMAT>      Output format: toml, json, yaml, cbor, msgpack, bincode
                            (auto-detected from file extension if not specified)
  --theme <THEME>            Syntax highlighting theme (default: base16-ocean.light)
  --list-themes              List all available syntax highlighting themes
  --no-counter               Disable automatic numbering of parts and slides
  --no-stats                 Disable presentation statistics display
  --wpm <WPM>                Speaking rate in words per minute (default: 150)
  --exclude-notes-from-duration  Exclude speaker notes from duration calculations
  -h, --help                 Print help information
```

## Folder Structure

Toboggan CLI processes a folder hierarchy to create presentations:

```
my-presentation/
├── _cover.md          # Cover slide (optional, contains title/date)
├── _footer.md         # Footer content for all slides (optional)
├── 01-intro/          # Part folder (becomes section divider)
│   ├── _part.md       # Part slide content
│   ├── slide1.md      # Regular slides
│   └── slide2.md
├── 02-main/
│   ├── _part.md
│   └── content.md
└── 99-conclusion.md   # Standalone slide
```

### Special Files

| File | Purpose |
|------|---------|
| `_cover.md` | Cover slide with presentation title and date |
| `_part.md` | Section divider slide within a folder |
| `_footer.md` | Footer content applied to all slides |

### Processing Rules

- Files are processed in **alphabetical order**
- Folders become **Part slides** (section dividers)
- Both `.md` and `.html` files are supported
- Hidden files (starting with `.`) are ignored
- Inline HTML tags (like `<abbr>`, `<mark>`) are preserved

## Frontmatter

Add TOML frontmatter to any slide using `+++` delimiters, the content should be TOML:

```markdown
+++
title = "Custom Slide Title"
skip = false              # Set to true to exclude from output
classes = ["centered", "dark"]  # CSS classes
css = "background: linear-gradient(...);"  # Inline CSS
css_file = "path/to/styles.css"  # External CSS file
grid = true               # Enable grid layout
duration = "2m 30s"       # Slide duration (or use seconds: 150)
+++

# Slide Content
Your content here...
```

### Cover Slide Frontmatter

The `_cover.md` file can set presentation-wide metadata:

```markdown
+++
title = "My Awesome Presentation"
date = "2025-03-15"
+++

# Welcome
Subtitle or opening content
```

## Advanced Features

### Progressive Reveals (Steps)

Use `<!-- pause -->` comments to reveal content step-by-step:

```markdown
# Key Points

First point appears immediately

<!-- pause -->
Second point appears on next step

<!-- pause: highlight -->
Third point appears with highlight class
```

### Grid Layouts

Create multi-column layouts with `<!-- cell -->` comments:

```markdown
+++
grid = true
+++

# Two Column Slide

<!-- cell -->
Left column content
- Point 1
- Point 2

<!-- cell: highlight -->
Right column content with highlight class
- Point A
- Point B
```

### Speaker Notes

Add presenter notes that won't be shown during presentation:

```markdown
# Main Slide Content

Your visible content here

<!-- notes -->
These are speaker notes:
- Remember to mention the demo
- Ask for questions
- Time check: should be at 10 minutes
```

### Code Blocks from Files

Include code from external files:

```markdown
# Code Example

Here's our implementation:

<!-- code:rust:src/main.rs -->

This will be replaced with the contents of src/main.rs
```

## Presentation Statistics

The CLI provides comprehensive statistics about your presentation:

### Overview Metrics
- Total slides and parts
- Word count (body + optional notes)
- Bullet points and images
- Estimated duration at your speaking rate

### Part Breakdown
Shows distribution of content across sections:
- Slides per part
- Words and percentage of total
- Estimated duration per part

### Duration Scenarios
Calculates presentation length for different speaking rates:
- Slow (110 WPM)
- Normal (150 WPM)
- Fast (170 WPM)
- Custom (your --wpm setting)
- Additional time for images (5 seconds each)

### Recommendations

The tool provides smart recommendations when:

| Condition | Recommendation |
|-----------|----------------|
| Duration > 50 minutes | Consider splitting into multiple presentations |
| Duration < 2 minutes | Presentation might be too short |
| One part > 50% of content | Consider splitting that part |
| > 100 words/slide average | High density - use more slides with less text |
| < 20 words/slide average | Low density - slides might need more content |

## Live Development with Bacon

For live updates while editing your presentation, use [bacon](https://dystroy.org/bacon/):

### Setup

Create a `bacon.toml` in your project root:

```toml
default_job = "toboggan"

[jobs.toboggan]
command = ["toboggan-cli", "./slides/", "--output", "presentation.toml"]
need_stdout = true
allow_warnings = true
default_watch = false
watch = ["slides"]  # Watch your presentation folder
```

### Usage

```bash
# Install bacon if needed
cargo install bacon

# Run with live reload
bacon

# Or run specific job
bacon toboggan
```

Now your `presentation.toml` will automatically rebuild whenever you edit files in the `slides/` folder!

## Output Formats

Toboggan CLI supports multiple output formats:

| Format | Extension | Description |
|--------|-----------|-------------|
| TOML | `.toml` | Default, human-readable |
| JSON | `.json` | Web-friendly, readable |
| YAML | `.yaml`, `.yml` | Alternative readable format |
| CBOR | `.cbor` | Compact binary, standardized |
| MessagePack | `.msgpack` | Ultra-compact binary |
| Bincode | `.bincode`, `.bin` | Rust-native, fastest |

Format is auto-detected from file extension or specify with `-f`:

```bash
# Auto-detect from extension
toboggan-cli slides/ -o presentation.json

# Explicit format
toboggan-cli slides/ -f yaml -o output.txt
```

## Examples

### Basic Presentation

```bash
# Create structure
mkdir -p my-talk/01-intro

# Create cover
cat > my-talk/_cover.md << 'EOF'
+++
title = "Introduction to Rust"
date = "2025-03-15"
+++

# Welcome to Rust Programming
EOF

# Create part
echo "# Chapter 1: Getting Started" > my-talk/01-intro/_part.md

# Create slide
cat > my-talk/01-intro/hello.md << 'EOF'
# Hello World

<!-- pause -->
```rust
fn main() {
    println!("Hello, world!");
}
```

<!-- notes -->
Explain that println! is a macro, not a function
EOF

# Convert
toboggan-cli my-talk/ -o presentation.toml
```

### Batch Processing

```bash
#!/bin/bash
# Convert all presentations in a directory

for dir in presentations/*/; do
  name=$(basename "$dir")
  toboggan-cli "$dir" \
    --date "$(date +%Y-%m-%d)" \
    --wpm 130 \
    -o "output/${name}.toml"
done
```

### CI/CD Integration

```yaml
# GitHub Actions example
- name: Build Presentations
  run: |
    for dir in presentations/*/; do
      toboggan-cli "$dir" -f json -o "dist/$(basename "$dir").json"
    done
```

## Troubleshooting

### Common Issues

**Missing syntax highlighting**
- Use `--list-themes` to see available themes
- Specify language in code blocks: ` ```rust`

**Incorrect duration estimates**
- Adjust `--wpm` to match your speaking pace
- Use `--exclude-notes-from-duration` if notes are just reminders

**Files processed in wrong order**
- Prefix with numbers: `01-intro.md`, `02-main.md`
- Use folders for logical grouping

## Contributing

Contributions welcome! Please see the [main repository](https://github.com/ilaborie/toboggan) for guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
