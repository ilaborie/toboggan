# toboggan-tui

Present from a terminal. A [ratatui] client that renders the deck as text, with
speaker notes, the next slide, and a slide list beside it.

Good over ssh, good on a second machine, and good when you want a presenter view
without a second browser window.

[ratatui]: https://ratatui.rs/

> [!IMPORTANT]
> Run it with `toboggan tui`. The crate also builds a standalone
> `toboggan-tui` binary from `src/main.rs`, kept from before the unified
> command; it takes only `--host` and `--port`.

```bash
toboggan -p my-talk          # in one terminal: the server
toboggan tui                 # in another: this client

toboggan tui --host 192.168.1.20 --presenter-token s3cret
```

## Keys

| Key | Does |
| --- | --- |
| `Space`, `↓`, `PageDown` | Next step — moves to the next slide when reveals run out |
| `↑`, `PageUp`, `Backspace` | Previous step |
| `→` / `←` | Next / previous **slide** |
| `Home` / `End` | First / last slide |
| digits then `Enter` | Go to that slide number |
| `b` | Blink — flash every other client, to get the room's attention |
| `l` | Show the log |
| `h`, `?` | Help |
| `Esc` | Close help or an error |
| `q`, `Ctrl+C` | Quit |

`PageUp`/`PageDown` are what a physical presenter remote emits, and they are
bound to the *step* commands on purpose: a remote should walk the whole deck,
reveals included, rather than skip past them.

Typing a number and pressing `Enter` means a deck is not limited to the nine
slides a single keystroke could reach.

## Roles

Connecting from the machine running the server presents, with no ceremony.
Connecting across the network needs `--presenter-token` to do anything more than
watch — see [SECURITY.md](../SECURITY.md).

## License

MIT or Apache-2.0, at your option.
