+++
title = "Mermaid options"
classes = ["no_title", "wide"]
+++

# Mermaid options

Tune one diagram from its fence — `mermaid:key=value,key=value`:

| Parameter | Values |
| --- | --- |
| `theme` | `default`, `dark`, `forest`, `neutral`, `modern` |
| `background` | `transparent` (default), `theme`, a colour |
| `width` | `60%`, `8cm`, … |
| `nodeSpacing`, `rankSpacing`, `aspectRatio`, `maxLabelWidth` | layout |
| `alt`, `class` | accessible label, extra CSS class |

<!-- pause -->

Deck-wide defaults live in a Mermaid JSON config, named from `toboggan.toml`:

```toml
[build]
mermaid-config = "mermaid.json"
```

<!-- notes -->

A fence's own parameters win over the config file, and Mermaid's
`%%{init: {…}}%%` still works inside the diagram itself.

An unknown parameter is an error, not a silent no-op — the same rule the front
matter and `toboggan.toml` already follow, for the same reason: a typo that does
nothing is only discovered on stage.

`background` defaults to `transparent` rather than Mermaid's opaque page colour,
because a themed slide almost never wants a white rectangle punched into it.
Pass `background=theme` to get Mermaid's own back.

The config file is Mermaid's own shape — `theme`, `themeVariables`,
`preferredAspectRatio`, `flowchart` — so a `mermaid.json` from elsewhere works
here unchanged.
