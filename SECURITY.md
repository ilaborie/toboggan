# Security

## Reporting a vulnerability

Open a [private security advisory][advisory] on this repository. Please do not
open a public issue for anything exploitable.

This is a hobby project with no service behind it and no SLA. Expect a reply
when the maintainer next sits down with it — but do report, because the thing
this software does is run on a laptop on a conference network.

## What the server actually does

Two facts shape everything below.

1. **`/api/terminal` spawns a real shell.** The embedded terminals run
   `$SHELL -ic <command>` in a working directory, both taken from the slide, on
   the machine running the server. That is the feature: a slide can show a live
   demo. It is also, from the network's point of view, remote code execution
   with the presenter's privileges.
2. **`POST /api/command` and the WebSocket move the deck.** Anyone who can send
   a command can change what the room is looking at.

So the server does not treat every connection alike.

## The rule

> **A connection from the machine running the server presents. A connection
> from anywhere else presents only if it carries the presenter token.**

| | `/`, `/run`, `/api/talk`, `/api/slides` | navigation (`/api/command`, WS commands) | `/api/terminal`, `/api/clients` |
| --- | :-: | :-: | :-: |
| default bind (`127.0.0.1`) | ✅ | ✅ | ✅ |
| `--host 0.0.0.0`, no token | ✅ | ✗ 403 | ✗ 403 |
| `--host 0.0.0.0`, with the token | ✅ | ✅ | ✅ |

The deck itself is public in every configuration — the room is meant to be able
to read the slides. What the room cannot do is drive them or open a shell.

**The default bind is loopback**, so out of the box the server is reachable only
from your own machine and every client presents, with no token and no ceremony.
The token only becomes relevant once you have deliberately opened `--host`.

### Letting a second device present

```bash
toboggan -p my-talk --host 0.0.0.0 --presenter-token "$(openssl rand -hex 16)"
```

Then, from the phone or the second laptop:

```
http://<your-ip>:8080/run?token=<the token>
```

The same string works for `toboggan tui --presenter-token …`, and can be sent as
`Authorization: Bearer <token>` instead of in the query string. It can also live
in `toboggan.toml` under `[serve] presenter-token`, or in
`TOBOGGAN_PRESENTER_TOKEN`.

A client *offers* a token; it never claims a role. The server decides what a
connection may do and reports the result in the `Registered` notification, so a
client that lies about itself gains nothing.

Starting with `--host` open and no token is legal, and logs a warning saying the
room is read-only. That is a supported configuration: it is how you show the deck
to people without handing them the controls.

## Limitations you should know about

- **Behind a reverse proxy, every connection arrives from the proxy** — usually
  loopback — so every client would be treated as local and would present. If you
  put Toboggan behind nginx, Caddy, or a tunnel, **set a presenter token**;
  `X-Forwarded-For` is deliberately not trusted, because a header the client
  controls cannot be the basis for a privilege decision.
- **The token is compared in constant time, but travels in plaintext.** There is
  no TLS: this is an HTTP server for a room, not for the internet. On a hostile
  network, anyone who can read the traffic can read the token — and can also
  read the deck, which is the part that matters less.
- **There is no rate limiting** on token attempts.
- **A slide is trusted input.** The deck author can put arbitrary HTML in a
  slide and arbitrary commands in a terminal directive, and both will run. Treat
  someone else's deck folder the way you would treat their shell script.
- **`--allowed-origins` widens CORS.** The default already allows any origin to
  *read* the deck; adding origins does not bypass the presenter check, but it is
  worth knowing that the read surface is open by design.

## Scope

In scope: anything that lets a client which should be audience-only drive the
deck, read something outside the deck folder, or run a command; anything that
lets a crafted deck escape the folder it was built from; and denial of service
against the server from a single connection.

Out of scope: the plaintext transport and the missing rate limit named above,
and the fact that a deck's own author can run commands — that is the product.

[advisory]: https://github.com/ilaborie/toboggan/security/advisories/new
