+++
title = "pause — stepped reveals"
classes = ["no_title"]
+++

# Stepped reveals

Split a slide into steps with a `pause` comment:

```markdown
First point.

<!-- pause -->

Second point — revealed on the next arrow.

<!-- pause :highlight -->

Add CSS classes to a step after a colon.
```

<!-- pause -->

This paragraph is step 2 — it appeared when you pressed <kbd>→</kbd>.

<!-- pause -->

…and this is step 3.

<!-- notes -->
The fenced block above shows the syntax verbatim; the bare `<!-- pause -->`
comments below it actually create the steps on this very slide.
