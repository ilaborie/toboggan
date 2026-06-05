+++
title = "notes — speaker notes"
classes = ["no_title"]
+++

# Speaker notes

Everything after a `notes` comment is hidden from the slide and kept for the
presenter view:

```markdown
# Visible heading

Audience sees this.

<!-- notes -->

Only the presenter sees this reminder.
```

<!-- pause -->

> [!TIP]
> Notes are excluded from the slide but still counted in the duration
> estimate — unless you pass `--exclude-notes-from-duration` to the CLI.

<!-- notes -->
This slide has real notes too: you are reading them in the presenter view.
