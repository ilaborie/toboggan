"""Network calls release the GIL.

The bindings run a tokio runtime inside the class and `block_on` it from
`#[pymethods]`. That is the right shape for a synchronous Python API, and it has
one trap: a `block_on` that keeps the GIL freezes every other Python thread for
its whole duration. Connecting, waiting for registration (bounded at five
seconds) and listing clients all used to do exactly that.

Two kinds of check here. The watchdog tests bound the damage during ordinary
calls — cheap, but weak on a local server, where a round trip is a couple of
milliseconds and a held GIL looks much like a released one. The last test is the
discriminating one: it makes the server slow on purpose, which is the only
condition under which this bug was ever visible.
"""

import os
import subprocess
import sys
import threading
import time

import pytest

# Comfortably longer than any scheduling hiccup on a loaded CI runner,
# comfortably shorter than the freeze this guards against.
MAX_STARVATION = 1.0

# How long the unresponsive-server probe watches for. Long enough that a held
# GIL is unmistakable, short enough to keep the suite quick.
PROBE_SECONDS = 2.0


class Watchdog:
    """A thread that times its own loop, reporting the longest gap it saw."""

    def __init__(self):
        self.worst = 0.0
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def _run(self):
        previous = time.monotonic()
        while not self._stop.is_set():
            now = time.monotonic()
            self.worst = max(self.worst, now - previous)
            previous = now
            time.sleep(0.001)

    def __enter__(self):
        self._thread.start()
        return self

    def __exit__(self, *_):
        self._stop.set()
        self._thread.join(timeout=5)


def test_listing_clients_does_not_freeze_other_threads(presenter):
    with Watchdog() as watchdog:
        for _ in range(30):
            presenter.clients()

    assert watchdog.worst < MAX_STARVATION, (
        f"another thread was starved for {watchdog.worst:.3f}s during clients()"
    )


def test_navigating_does_not_freeze_other_threads(presenter):
    with Watchdog() as watchdog:
        for _ in range(30):
            presenter.next()

    assert watchdog.worst < MAX_STARVATION, (
        f"another thread was starved for {watchdog.worst:.3f}s during next()"
    )


# Run in a child process, not here. If the GIL is *not* released the connecting
# thread never gives it up — no timeout, no interrupt, nothing Python can do —
# so an in-process version of this test does not fail, it wedges the whole
# session. Isolating it means the regression shows up as a killed child and a
# readable assertion instead of a CI job that runs until the platform times out.
PROBE = r'''
import socket, sys, threading, time
from toboggan_py import Toboggan

# Never accept()ed: the listen backlog completes the TCP handshake by itself, so
# the client connects and then waits for a reply that never comes.
silent = socket.socket()
silent.bind(("127.0.0.1", 0))
silent.listen(1)
port = silent.getsockname()[1]

threading.Thread(target=lambda: Toboggan("127.0.0.1", port), daemon=True).start()

worst, previous, deadline = 0.0, time.monotonic(), time.monotonic() + float(sys.argv[1])
while time.monotonic() < deadline:
    now = time.monotonic()
    worst = max(worst, now - previous)
    previous = now
    time.sleep(0.001)

print(worst)
'''


def test_connecting_to_an_unresponsive_server_does_not_freeze_the_interpreter():
    """The case that actually bit: a server that is reachable but never answers.

    On a local server every call is a couple of milliseconds, far too fast to
    tell a held GIL from a released one — which is why the watchdog tests above
    pass either way. This makes the wait long on purpose.
    """
    try:
        finished = subprocess.run(
            [sys.executable, "-c", PROBE, str(PROBE_SECONDS)],
            capture_output=True,
            text=True,
            timeout=PROBE_SECONDS + 20,
            env={**os.environ, "PYTHONUNBUFFERED": "1"},
        )
    except subprocess.TimeoutExpired:
        pytest.fail(
            "the interpreter never came back while a thread waited on an "
            "unresponsive server — the GIL is being held across the wait"
        )

    assert finished.returncode == 0, f"the probe failed: {finished.stderr[-2000:]}"

    worst = float(finished.stdout.strip().splitlines()[-1])
    assert worst < MAX_STARVATION, (
        f"the main thread was frozen for {worst:.3f}s while another thread "
        f"waited on an unresponsive server"
    )
