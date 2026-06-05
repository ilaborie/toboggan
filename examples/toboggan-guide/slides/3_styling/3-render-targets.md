+++
title = "Render targets"
classes = ["no_title", "wide"]
+++

# One deck, two outputs

`hidden_in` drops a slide from a specific render target:

```toml
+++
hidden_in = ["pdf"]   # web only — e.g. a live terminal
+++
```
```toml
+++
hidden_in = ["web"]   # pdf only — a static screenshot twin
+++
```

<!-- pause -->

> [!TIP]
> Pattern: keep a **live `term`** slide for the web (`hidden_in = ["pdf"]`),
> paired with a **static code** twin for the PDF (`hidden_in = ["web"]`).
> The audience sees the demo; the handout still has the output.

You'll see this exact pair in the **Server** section →
