+++
title = "Resources"
classes = ["no_title", "center"]
style = """
background: radial-gradient(60vw 60vw at 50% 30%, rgba(76, 201, 240, 0.16), transparent 60%);
"""
+++

<style>
  .next-steps { text-align: left; display: inline-block; }
  .next-steps code { font-size: 0.85em; }
</style>

# Go build a deck

<div class="next-steps">

```console
$ git clone https://github.com/ilaborie/toboggan
$ cd toboggan
$ mise build:web                 # the embedded web client
$ cargo install --path toboggan  # one binary, every command
$ cd examples/toboggan-guide
$ toboggan -p ./slides/          # build + serve, live reload
```

</div>

<!-- pause -->

- 📂 This deck's source: `examples/toboggan-guide/`
- 🧪 Feature reference: `examples/demo-terminal/`
- 📖 API docs: `/doc` on a running server

### Happy sledding 🛝
