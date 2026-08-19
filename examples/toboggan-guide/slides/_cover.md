+++
title = "Toboggan — User Guide"
# Pinned on purpose: `toboggan-guide.toml` is embedded into the server with
# `include_str!`, so an unpinned date restamps the committed artifact on every
# rebuild and no CI check can tell a real edit from the clock moving.
date = "2026-08-17"
classes = ["no_title", "cover"]
style = """
min-height: 100%;
width: 100%;
overflow: hidden;
background:
  radial-gradient(60vw 60vw at 78% 18%, rgba(76, 201, 240, 0.22), transparent 60%),
  radial-gradient(50vw 50vw at 18% 88%, rgba(255, 140, 66, 0.18), transparent 60%);
"""
+++

<style>
  .hero {
    text-align: center;
    line-height: 1.1;
  }
  .hero .sled {
    font-size: 4rem;
    display: block;
    margin-bottom: 0.2em;
    filter: drop-shadow(0 0.1em 0.3em rgba(0, 0, 0, 0.5));
  }
  .hero h1 {
    font-size: 4rem;
    margin: 0;
    background: linear-gradient(100deg, var(--accent), var(--accent3));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  .hero .tag {
    font-size: 1.15rem;
    color: rgba(230, 237, 245, 0.85);
    margin-top: 0.4em;
  }
  .hero .hint {
    margin-top: 2.5em;
    font-size: 0.85rem;
    color: rgba(230, 237, 245, 0.55);
  }
</style>

<div class="hero">
  <span class="sled">🛝</span>

# Toboggan

  <p class="tag">Markdown decks with live terminals — author once, present anywhere.</p>
  <p class="hint">Press <kbd>→</kbd> to start · built <em>with</em> Toboggan</p>
</div>
