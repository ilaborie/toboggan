"""The constructor always comes back.

`Toboggan(...)` used to have no upper bound at all. `WebSocketClient::connect`
cannot report failure — it reconnects forever by design — so against a server
that completes the TCP handshake and then never answers the upgrade, the
constructor waited for a reply that was never coming. `REGISTRATION_TIMEOUT`
existed precisely to stop that, and its doc said so, but it sits *after* the
connect and so was never reached.

Worse than slow: the wait happens with the GIL released, so `KeyboardInterrupt`
only sets a flag nobody checks. A REPL in that state has to be killed.

These tests need no server, which is why they live outside the `server`
fixture — a fixture that can skip is the wrong place for the test that proves
the binding cannot hang.
"""

import socket
import subprocess
import sys
import time

import pytest

# The constructor bounds the socket at 5s and each REST call at 30s. A closed
# port refuses immediately; the silent-server case has to wait both out. Well
# clear of either, and still far short of "hung".
MUST_RETURN_WITHIN = 90.0


def test_a_closed_port_is_a_connection_error():
    """Nothing is listening: the failure is immediate and it is a connection one.

    `ConnectionError` rather than a bare `RuntimeError` is the contract the stub
    promises, and it is what `conftest.remote_client` keys on to tell "this host
    cannot route to itself" apart from "the bindings are broken".
    """
    from toboggan_py import Toboggan

    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    # Closed again by the time we ask, so the OS refuses rather than queues.

    started = time.monotonic()
    with pytest.raises(ConnectionError):
        Toboggan("127.0.0.1", port)

    assert time.monotonic() - started < MUST_RETURN_WITHIN


# Run in a child process. If this regresses, the constructor does not fail — it
# blocks forever with the GIL released, so an in-process version would wedge the
# whole session rather than fail one test, and no pytest timeout can unwedge it.
# A killed child and a readable assertion is the difference between a red test
# and a CI job that runs until the platform gives up.
PROBE = r"""
import socket, sys, time
from toboggan_py import Toboggan

# Never accept()ed: the listen backlog completes the TCP handshake by itself, so
# the client connects and then waits on a reply that never comes. This is the
# shape of the bug — a *reachable* server that answers nothing.
silent = socket.socket()
silent.bind(("127.0.0.1", 0))
silent.listen(1)
port = silent.getsockname()[1]

started = time.monotonic()
try:
    Toboggan("127.0.0.1", port)
    outcome = "returned without error"
except Exception as raised:
    outcome = type(raised).__name__
print(f"{time.monotonic() - started:.3f} {outcome}")
"""


@pytest.mark.slow
def test_a_server_that_never_answers_still_ends_the_call():
    """The regression guard: reachable, silent, and the constructor comes back.

    Slow on purpose — it waits out the real timeouts rather than trusting a
    shortened copy of them, because a bound that is only exercised at a test
    value is a bound nobody has checked.
    """
    try:
        finished = subprocess.run(
            [sys.executable, "-c", PROBE],
            capture_output=True,
            text=True,
            timeout=MUST_RETURN_WITHIN,
        )
    except subprocess.TimeoutExpired:
        pytest.fail(
            f"the constructor did not return within {MUST_RETURN_WITHIN:.0f}s "
            f"against a server that accepts connections and answers nothing — "
            f"this is the hang the connect timeout exists to prevent"
        )

    assert finished.returncode == 0, f"the probe failed: {finished.stderr[-2000:]}"

    elapsed, _, outcome = finished.stdout.strip().splitlines()[-1].partition(" ")
    assert outcome != "returned without error", (
        "a server that never answered the upgrade produced a usable client"
    )
    assert float(elapsed) < MUST_RETURN_WITHIN
