"""Drive a running Toboggan presentation from Python.

Start a server first, e.g. `toboggan -p examples/riir-folder`.
"""

from time import sleep

from toboggan_py import Toboggan

# A client on the server's own machine always presents. Across the network,
# pass `presenter_token="…"` (or set TOBOGGAN_PRESENTER_TOKEN) to do more than
# watch — see SECURITY.md in the main repository.
tbg = Toboggan("localhost", 8080)

print(f"toboggan: {tbg}")
print(f"role: {tbg.role} (can drive the deck: {tbg.is_presenter})")

talk = tbg.talk
print(f"talk: {talk.title} — {talk.date} [{talk.lang or 'en'}]")
print(f"slides: {len(tbg.slides)}")

for index, slide in enumerate(tbg.slides, start=1):
    planned = f"{slide.duration:.0f}s" if slide.duration else "—"
    print(f"  {index:>3}. [{slide.kind}] {slide.title} ({planned})")

if not tbg.is_presenter:
    print("watching only: the server will refuse navigation commands")
    raise SystemExit(0)

print(f"state: {tbg.state}")

tbg.previous()
sleep(1)
print(f"state after previous: {tbg.state}")

tbg.next()
sleep(1)
print(f"state after next: {tbg.state}")

tbg.goto(3)
sleep(1)
state = tbg.state
print(f"state after goto(3): {state} (slide {state.slide}, step {state.step})")
print(f"on the last slide: {state.is_last_slide(len(tbg.slides))}")

for client in tbg.clients():
    print(f"connected: {client.name} ({client.role}) from {client.ip_addr}")
