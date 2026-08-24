"""Drive a running Toboggan presentation from Python.

Start a server first, e.g. `toboggan -p examples/riir-folder`, then:

    python example.py                 # localhost:8080
    python example.py localhost 8097  # somewhere else

Host and port are arguments rather than constants on purpose: a hardcoded
`localhost:8080` points this script at whatever deck happens to be live, which
is rarely the one you meant to poke at.

Nothing here prints diagnostics from the bindings themselves. They report over
Python's own `logging`, so add `logging.basicConfig(level=logging.DEBUG)` to
watch the socket, the deck reloads and clients coming and going.
"""

import os
import sys

from toboggan_py import Toboggan


def describe(tbg):
    """Everything the client knows before it touches anything."""
    print(f"toboggan: {tbg}")
    print(f"role: {tbg.role} (can drive the deck: {tbg.is_presenter})")

    talk = tbg.talk
    print(f"talk: {talk.title} — {talk.date} [{talk.lang or 'en'}]")
    print(f"slides: {len(tbg.slides)}")

    for index, slide in enumerate(tbg.slides, start=1):
        planned = f"{slide.duration:.0f}s" if slide.duration else "—"
        print(f"  {index:>3}. [{slide.kind}] {slide.title} ({planned})")

    print(f"state: {tbg.state}")


def drive(tbg):
    """Move the deck about.

    No sleeps: every navigation call returns once the server has applied it, so
    the state read on the next line is the state that call produced.
    """
    tbg.previous()
    print(f"state after previous: {tbg.state}")

    tbg.next()
    print(f"state after next: {tbg.state}")

    tbg.goto(3)
    state = tbg.state
    print(f"state after goto(3): {state} (slide {state.slide}, step {state.step})")
    print(f"on the last slide: {state.is_last_slide}")


def main():
    default_host = os.environ.get("TOBOGGAN_HOST", "localhost")
    default_port = os.environ.get("TOBOGGAN_PORT", 8080)

    host = sys.argv[1] if len(sys.argv) > 1 else default_host
    port = sys.argv[2] if len(sys.argv) > 2 else default_port

    # A client on the server's own machine always presents. Across the network,
    # pass `presenter_token="…"` (or set TOBOGGAN_PRESENTER_TOKEN) to do more
    # than watch — see SECURITY.md in the main repository.
    #
    # As a context manager: closing shuts the client's runtime down
    # deliberately, rather than leaving the garbage collector to do it while
    # holding the GIL.
    with Toboggan(host, int(port)) as tbg:
        describe(tbg)

        try:
            drive(tbg)
        except PermissionError as refused:
            # Non-zero. Exiting 0 here would report success for a deck that
            # never budged, which is precisely the habit these bindings were
            # fixed to break.
            raise SystemExit(f"watching only: {refused}") from None

        for client in tbg.clients():
            print(f"connected: {client.name} ({client.role}) from {client.ip_addr}")


if __name__ == "__main__":
    main()
