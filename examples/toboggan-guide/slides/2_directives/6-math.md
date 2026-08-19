+++
title = "Math"
classes = ["no_title", "wide"]
+++

# LaTeX math

Write LaTeX between dollar signs — `$…$` inline, `$$…$$` on its own line:

```markdown
Euler's identity is $e^{i\pi} + 1 = 0$.

$$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$$
```

<!-- pause -->

Euler's identity is $e^{i\pi} + 1 = 0$, and the quadratic formula:

$$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$$

<!-- pause -->

| Where | How it renders |
| --- | --- |
| Web, `/run`, exported HTML | MathML, generated while the deck builds |
| PDF and thumbnails | MiTeX, generated while typst compiles |

> [!NOTE]
> There is no JavaScript and no web font behind this — the math is part of the
> document, so an exported deck renders it with no network.

<!-- notes -->

A bad expression stops the build and names the file, rather than rendering as
nothing in front of an audience. That is the whole reason the conversion happens
here instead of in the browser.

MathML needs Chrome 109+, Firefox, or Safari — all current browsers render it
natively.
