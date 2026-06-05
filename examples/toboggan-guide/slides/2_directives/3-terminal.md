+++
title = "term — live terminals"
classes = ["no_title", "wide"]
+++

# Live terminals

Embed a real PTY in a slide. Syntax:

```markdown
<!-- term: . -->                      # shell in the current dir
<!-- term: ./src :light -->           # light theme
<!-- term: . | bacon test -->         # run a command on connect
```

The terminal below is **live** — click it and type:

<!-- term: . -->

<!-- notes -->
Multiple `term` comments on one slide render side by side. The shell is chosen
by the server's --shell flag. The fenced examples above stay as text; the bare
one below is a working terminal.
