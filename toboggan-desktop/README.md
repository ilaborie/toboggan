# toboggan-desktop

Present from a native window. An [iced] client that connects to a running
Toboggan server and stays in sync with every other client.

> [!IMPORTANT]
> Run it with `toboggan desktop`. The crate also builds a standalone
> `toboggan-desktop` binary from `src/main.rs`, kept from before the unified
> command; it takes no arguments and always uses the default server.

```bash
toboggan -p my-talk          # in one terminal: the server
toboggan desktop             # the client

toboggan desktop --host 192.168.1.20 --presenter-token s3cret
```

## Keys

| Key | Does |
| --- | --- |
| `Space`, `↓`, `PageDown` | Next step |
| `↑`, `PageUp`, `Backspace` | Previous step |
| `→` / `←` | Next / previous slide |
| `Home` / `End` | First / last slide |
| `b` | Blink |
| `s` | Toggle the sidebar |
| `F11` | Fullscreen |
| `h`, `?` | Toggle help |
| `Esc` | Close help or an error |
| `Cmd+Q` | Quit |

Each shortcut is described once, in `actions.rs`, and the help panel reads that
description — so the panel cannot drift from what the keys actually do. It used
to be twenty-eight hardcoded strings maintained separately from the handlers,
and by the time anyone checked, they disagreed.

## Roles

A client on the server's own machine presents. Across the network it needs
`--presenter-token`; see [SECURITY.md](../SECURITY.md).

## License

MIT or Apache-2.0, at your option.

[iced]: https://github.com/iced-rs/iced
