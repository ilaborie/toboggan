"""Drive a running Toboggan presentation from Python.

Start a server first, e.g. `toboggan -p examples/riir-folder`, then:

    python example.py                 # localhost:8080
    python example.py localhost 8097  # somewhere else

Host and port are arguments rather than constants on purpose: a hardcoded
`localhost:8080` points this script at whatever deck happens to be live, which
is rarely the one you meant to poke at.
"""

import os
import sys

from toboggan_py import Toboggan

host = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("TOBOGGAN_HOST", "localhost")
port = int(sys.argv[2] if len(sys.argv) > 2 else os.environ.get("TOBOGGAN_PORT", 8080))

# A client on the server's own machine always presents. Across the network,
# pass `presenter_token="…"` (or set TOBOGGAN_PRESENTER_TOKEN) to do more than
# watch — see SECURITY.md in the main repository.
tbg = Toboggan(host, port)

print(f"toboggan: {tbg}")
print(f"role: {tbg.role} (can drive the deck: {tbg.is_presenter})")

talk = tbg.talk
print(f"talk: {talk.title} — {talk.date} [{talk.lang or 'en'}]")
print(f"slides: {len(tbg.slides)}")

for index, slide in enumerate(tbg.slides, start=1):
    planned = f"{slide.duration:.0f}s" if slide.duration else "—"
    print(f"  {index:>3}. [{slide.kind}] {slide.title} ({planned})")

print(f"state: {tbg.state}")

# No sleeps below: every navigation call returns once the server has applied
# it, so the state read on the next line is the state that call produced.
try:
    tbg.previous()
    print(f"state after previous: {tbg.state}")

    tbg.next()
    print(f"state after next: {tbg.state}")

    tbg.goto(3)
    state = tbg.state
    print(f"state after goto(3): {state} (slide {state.slide}, step {state.step})")
    print(f"on the last slide: {state.is_last_slide(len(tbg.slides))}")
except PermissionError as refused:
    print(f"watching only: {refused}")
    raise SystemExit(0) from None

for client in tbg.clients():
    print(f"connected: {client.name} ({client.role}) from {client.ip_addr}")
