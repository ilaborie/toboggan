# toboggan-stats

Counts the things a presentation is made of: words, bullets, images, reveal
steps — and estimates how long the deck will take to say out loud.

Used by `toboggan stats`, by [`toboggan-lint`](../toboggan-lint) for the rules
with budgets, and by the terminal client's progress display.

## What it gives you

| Item | Role |
| --- | --- |
| `SlideStats` | `words`, `bullets`, `images`, `steps`, `notes_words` for one slide |
| `PresentationStats` | The same across a deck, plus per-slide detail |
| `DurationEstimate` | Speaking time at a given words-per-minute |
| `HtmlDocument` | A parsed slide body — parse once, ask many questions |
| `CodeBlock` | A code block found in a slide, with its language |
| `count_words`, `count_images_in_html`, … | The individual counters |

```rust,ignore
use toboggan_stats::SlideStats;

let stats = SlideStats::from_slide(&slide);
println!("{} words, {} steps", stats.words, stats.steps);
```

## Parse once

`HtmlDocument` exists because the counters used to each parse the slide body
independently — with three passes per slide (title, body, notes) that came to
roughly ten [scraper] parses for every slide, and the linter, which asks more
questions, reached about twenty-five. Parse the body once and hand the document
to whatever needs to inspect it.

[scraper]: https://github.com/causal-agent/scraper

## Duration

Estimates come from a words-per-minute rate (`--wpm`, default 150). Speaker
notes can be included or excluded, because whether you read your notes aloud is
a fact about you rather than about the deck.

This is separate from a slide's `duration` front matter, which is what the author
*planned* — the presenter view compares the two and tells you which way you are
drifting.

## License

MIT or Apache-2.0, at your option.
