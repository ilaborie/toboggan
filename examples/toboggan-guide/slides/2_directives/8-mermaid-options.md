+++
title = "Mermaid options"
classes = ["no_title", "wide"]
+++

# Mermaid options

Tune one diagram from its fence — `mermaid:key=value,key=value`:

| Parameter | Values |
| --- | --- |
| `theme` | `default`, `dark`, `forest`, `neutral`, `modern` |
| `background` | `transparent` (default), `theme`, `#1e293b`, `slategray` |
| `width` | `60%`, `8cm` — units CSS and Typst share |
| `nodeSpacing`, `rankSpacing`, `aspectRatio`, `maxLabelWidth` | layout |
| `fastText` | reproducible flowchart label widths (on by default) |
| `alt`, `class` | accessible label (write it last), extra CSS class (HTML only) |

<!-- pause -->

Deck-wide defaults live in a Mermaid JSON config, named from `toboggan.toml`:

```toml
[build]
mermaid-config = "mermaid.json"
```

<!-- notes -->

A fence's own parameters win over the config file. Mermaid's in-diagram
`%%{init: {…}}%%` is accepted and ignored — the renderer only applies it from
its own CLI, which is not the code path here.

`width` takes only the units CSS and Typst both understand — `%`, `pt`, `mm`,
`cm`, `in`, `em`. `px` is refused rather than accepted and then dropped, so a
deck cannot look right on the projector and fail to export.

An unknown parameter — or an unknown value for one — is an error, not a silent
no-op: the same rule the front matter and `toboggan.toml` already follow, for
the same reason, that a typo which does nothing is only discovered on stage.
A misspelled `background` colour fails the build rather than being painted,
which for an invalid paint means black. `class` and `alt` are the two free-text
values, so they are the two not checked; `alt` runs to the end of the fence, so
its label can hold commas as long as you write it last.

`background` defaults to `transparent` rather than Mermaid's opaque page colour,
because a themed slide almost never wants a white rectangle punched into it.
Pass `background=theme` to get Mermaid's own back.

The config file is Mermaid's own shape — `theme`, `themeVariables`,
`preferredAspectRatio`, `flowchart` — so a `mermaid.json` from elsewhere works
here unchanged. It is held to the same rule as a fence: an unknown setting, or a
theme name that is not one of the seven, fails the build instead of being
dropped on the floor.
