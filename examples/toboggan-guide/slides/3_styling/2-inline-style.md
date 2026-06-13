+++
title = "Per-slide style"
classes = ["no_title", "wide"]
style = """
background:
  radial-gradient(40vw 40vw at 85% 20%, rgba(255, 209, 102, 0.22), transparent 60%),
  linear-gradient(160deg, #0d1b2a, #15314a);
"""
+++

<style>
  .card-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1em;
    margin-top: 0.5em;
  }
  .card {
    border: 1px solid rgba(76, 201, 240, 0.35);
    border-radius: 0.6em;
    padding: 0.8em 1em;
    background: rgba(76, 201, 240, 0.08);
  }
  .card h3 { margin: 0 0 0.2em; }
</style>

# Per-slide CSS

The `style` frontmatter paints the slide background; a `<style>` block scopes
component CSS to this slide only.

<div class="card-grid">
  <div class="card">

### Frontmatter `style`

A full-bleed gradient, set right in the TOML header.

  </div>
  <div class="card">

### Inline `<style>`

These two cards — laid out with a local CSS grid.

  </div>
</div>

<!-- notes -->
Theme variables like var(--accent) are available here, so per-slide CSS stays
consistent with the global palette in public/style.css.
