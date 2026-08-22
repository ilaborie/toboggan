"""A move another client made shows up here.

This is the socket's whole remaining job. Commands travel over REST and answer
their caller, so the only thing left that *must* arrive over the WebSocket is
somebody else's move — and nothing tested it, because every other assertion in
this suite reads state its own command just wrote. A completely broken
`handle_state` would have left the suite green.

The waits here are real waits, not a `sleep` smuggled back in: a push has no
caller to return to, so there is nothing to be synchronous with. That is exactly
the distinction `test_sync.py` is about — a command you sent is done when it
returns; a move someone else made arrives when it arrives.
"""

import time

from conftest import PRESENTER_TOKEN, remote_client, requires_lan

# Generous: a loopback push is sub-millisecond, and a runner under load is not a
# reason to fail. Still far short of "never arrived".
PUSH_TIMEOUT = 10.0


def until(predicate, timeout=PUSH_TIMEOUT):
    """Waits for a pushed update, returning what the observer last saw."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        seen = predicate()
        if seen is not None:
            return seen
        time.sleep(0.01)
    return None


@requires_lan
def test_a_watcher_sees_a_move_it_did_not_make(server, presenter):
    """The push path, end to end.

    `presenter` never sends a command here — everything it learns, it learns
    from the socket.
    """
    _, port = server
    remote = remote_client(port, presenter_token=PRESENTER_TOKEN)

    remote.goto(2)
    assert until(lambda: presenter.state.slide == 2 or None), (
        f"the watcher still reports slide {presenter.state.slide} after another "
        f"client moved the deck to 2 — nothing is arriving over the socket"
    )

    remote.goto(5)
    assert until(lambda: presenter.state.slide == 5 or None), (
        f"the watcher stuck at {presenter.state.slide} after a second move"
    )


@requires_lan
def test_a_watchers_own_command_still_wins(server, presenter):
    """The race the in-flight guard is for.

    A pushed frame and this client's own command can be in flight at once, and
    the server broadcasts before it answers — so the echo of an earlier move can
    arrive *after* a later command has already written the cache. Navigation
    promises the state it returns is the state it produced; that has to hold
    even with another client moving the deck at the same time.
    """
    _, port = server
    remote = remote_client(port, presenter_token=PRESENTER_TOKEN)

    for round_number in range(20):
        remote.goto(2)
        presenter.goto(7)
        # Read immediately, with the remote's frames still arriving: the value
        # a navigation call produced must not be overwritten by an older echo.
        assert presenter.state.slide == 7, (
            f"round {round_number}: goto(7) returned, then state reported "
            f"{presenter.state.slide} — an echo overwrote the command's answer"
        )


def test_the_deck_is_not_reported_stale_when_nothing_went_wrong(presenter):
    """The staleness flag stays off in the ordinary case.

    It is set only when a deck reload arrives and the refetch fails; a plain
    session must never trip it, or every getter starts raising.
    """
    presenter.goto(1)
    assert presenter.talk is not None
    assert presenter.slides is not None
    assert presenter.state.slide == 1
