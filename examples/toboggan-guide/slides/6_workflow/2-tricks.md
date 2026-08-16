+++
title = "Tricks of the trade"
classes = ["no_title", "wide"]
+++

# Tricks of the trade

<div class="tricks">

### 🔤 Custom fonts
Drop `<link>` tags in `_head.html` and point `--font-slide-*` at them in your
stylesheet — exactly how this deck loads Space Grotesk.

### 📊 Live footer
The footer reads `--current-slide` / `--total-slides`; this guide's 🛝 rides the
progress bar with pure CSS, no assets.

### 🖼️ Pre-render diagrams
Compile D2 / Mermaid / Graphviz to `.webp` in `public/`, embed with `<img>`.
Keeps the `.toml` small and the render fast.

### 📐 `spread-steps`
Combine with `<!-- pause -->` to space reveals evenly down a tall slide.

</div>

<style>
  .tricks {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.8em 1.6em;
    text-align: left;
    margin-top: 0.4em;
  }
  .tricks h3 { margin: 0; }
  .tricks p { margin: 0.15em 0 0; font-size: 0.9em; }
</style>
