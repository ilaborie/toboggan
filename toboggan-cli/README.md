# Toboggan CLI

A command-line interface for creating and converting Toboggan presentation files from Markdown sources.

[![Crates.io](https://img.shields.io/crates/v/toboggan-cli.svg)](https://crates.io/crates/toboggan-cli)
[![Documentation](https://docs.rs/toboggan-cli/badge.svg)](https://docs.rs/toboggan-cli)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/ilaborie/toboggan)

## Overview

The Toboggan CLI converts Markdown files and structured folder hierarchies into TOML configuration files for the Toboggan presentation system. It supports both simple flat Markdown files and complex folder-based organizations.

## Installation

### From Cargo

```bash
cargo install toboggan-cli
```

### From Source

```bash
git clone https://github.com/ilaborie/toboggan
cd toboggan/toboggan-cli
cargo install --path .
```

## Quick Start

### Convert a Markdown file

```bash
toboggan-cli presentation.md -o talk.toml
```

### Process a folder structure

```bash
toboggan-cli slides-folder/ -o talk.toml
```

### Override title and date

```bash
toboggan-cli slides/ --title "My Conference Talk" --date "2024-12-31"
```

## Input Formats

### Flat Markdown Files

Use horizontal rules (`---`) to separate slides:

```markdown
# My Presentation

## Introduction
Welcome to my talk about Rust!

---

### Key Benefits
- Memory safety
- Zero-cost abstractions
- Fearless concurrency

> Remember to mention the borrow checker

---

### Getting Started
Let's dive into some code examples...
```

**Slide Type Rules:**
- `#` (H1) → Presentation title
- `##` (H2) → Part slides (section dividers)
- `###`+ (H3+) → Standard content slides
- `>` (Blockquotes) → Speaker notes

### Folder Structure

For complex presentations, organize content in folders:

```
my-talk/
├── title.md              # Presentation title
├── date.txt              # Date in YYYY-MM-DD format
├── _cover.md             # Cover slide
├── 01-intro/             # Section folder
│   ├── _part.md          # Section divider
│   ├── 01-overview.md    # Content slides
│   └── 02-goals.md
├── 02-content/
│   ├── _part.md
│   ├── slide1.md
│   └── slide2.html       # HTML slides supported
└── 99-conclusion.md      # Final slide
```

**Special Files:**
- `title.md`/`title.txt` → Presentation title
- `date.txt` → Presentation date (YYYY-MM-DD)
- `_cover.md` → Cover slide content
- `_part.md` → Section divider content

**Processing Rules:**
- Files processed in alphabetical order
- Folders become Part slides
- `.md` and `.html` files supported
- Hidden files (starting with `.`) ignored
- Markdown converted to HTML with alt text

## Command-Line Options

```
toboggan-cli [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input file or folder to process

Options:
  -o, --output <OUTPUT>  Output file (default: stdout)
  -t, --title <TITLE>    Title override (takes precedence over files)
  -d, --date <DATE>      Date override in YYYY-MM-DD format
  -h, --help             Print help
```

## Examples

### Simple Presentation

```bash
# Create a markdown file
cat > slides.md << 'EOF'
# My Talk

## Introduction
Welcome to my presentation

---

### Key Points
- Point 1
- Point 2

> Don't forget the demo
EOF

# Convert to TOML
toboggan-cli slides.md -o presentation.toml
```

### Folder-Based Presentation

```bash
# Create folder structure
mkdir -p my-talk/01-intro
echo "My Amazing Talk" > my-talk/title.md
echo "2024-03-15" > my-talk/date.txt
echo "# Welcome" > my-talk/_cover.md
echo "## Chapter 1" > my-talk/01-intro/_part.md
echo "### Overview" > my-talk/01-intro/overview.md

# Convert to TOML
toboggan-cli my-talk/ -o talk.toml
```

### Dynamic Content

```bash
# Use current date and dynamic title
toboggan-cli slides/ \
  --title "$(date '+%Y Conference Talk')" \
  --date "$(date '+%Y-%m-%d')"

# Batch processing
for dir in talks/*/; do
  name=$(basename "$dir")
  toboggan-cli "$dir" -o "output/${name}.toml"
done
```

## Integration

### Build Systems

```bash
# Makefile integration
presentations/%.toml: presentations/%/
	toboggan-cli $< -o $@

# GitHub Actions
- name: Build presentations
  run: |
    find presentations/ -type d -name "*/" | while read dir; do
      toboggan-cli "$dir" -o "dist/$(basename "$dir").toml"
    done
```

### Scripting

```bash
#!/bin/bash
# generate-presentations.sh

set -euo pipefail

SLIDES_DIR="${1:-slides}"
OUTPUT_DIR="${2:-output}"

mkdir -p "$OUTPUT_DIR"

for presentation in "$SLIDES_DIR"/*/; do
  if [[ -d "$presentation" ]]; then
    name=$(basename "$presentation")
    toboggan-cli "$presentation" \
      --date "$(date '+%Y-%m-%d')" \
      -o "$OUTPUT_DIR/${name}.toml"
    echo "Generated: $OUTPUT_DIR/${name}.toml"
  fi
done
```

## Output Format

The CLI generates TOML files compatible with the Toboggan presentation system:

```toml
date = "2024-01-26"

[title]
type = "Text"
text = "My Presentation"

[[slides]]
kind = "Cover"
style = []

[slides.title]
type = "Text"
text = "Welcome"

[slides.body]
type = "Html"
raw = "<p>Welcome to my presentation</p>"
alt = "Welcome to my presentation"

[slides.notes]
type = "Text"
text = "Remember to speak slowly"
```

## Library Usage

The CLI can also be used as a library:

```rust
use toboggan_cli::{Settings, launch};
use std::path::PathBuf;

let settings = Settings {
    output: Some(PathBuf::from("output.toml")),
    title: Some("Generated Talk".to_string()),
    date: Some("2024-12-31".to_string()),
    input: PathBuf::from("slides/"),
};

launch(settings)?;
```

## Error Handling

The CLI provides clear error messages for common issues:

- **Invalid date format**: `Date must be in YYYY-MM-DD format (e.g., 2024-12-31)`
- **File not found**: `Failed to read file: No such file or directory`
- **Invalid Markdown**: `Failed to parse markdown: unexpected token`
- **Permission denied**: `Permission denied when writing to output file`

## Contributing

Contributions are welcome! Please see the [main repository](https://github.com/ilaborie/toboggan) for contribution guidelines.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.