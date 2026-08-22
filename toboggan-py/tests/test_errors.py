"""A command the server rejects raises, rather than quietly doing nothing.

And it raises the *right* thing. Every API failure used to collapse into
`ConnectionError`, which is how a deserialization bug in `clients()` stayed
hidden: it reached Python dressed as an unreachable server, a long way from the
line that caused it. The tests here pin the three cases apart so the next bug of
that class cannot wear the same disguise.
"""

import http.server
import threading

import pytest


def test_a_slide_the_deck_does_not_have_raises(presenter):
    presenter.goto(2)
    before = presenter.state.slide

    with pytest.raises(RuntimeError):
        presenter.goto(9999)

    assert presenter.state.slide == before


def test_blink_succeeds_and_moves_nothing(presenter):
    presenter.goto(3)
    before = (presenter.state.slide, presenter.state.step)

    presenter.blink()

    assert (presenter.state.slide, presenter.state.step) == before


def test_an_out_of_range_index_is_not_a_connection_problem(presenter):
    """`RuntimeError`, not `ConnectionError`: the server was reachable and
    answered. Reporting it as a connection fault sends a reader hunting for a
    network problem that is not there."""
    with pytest.raises(RuntimeError) as raised:
        presenter.goto(10_000)

    # `raised.type`, not `isinstance`: `ConnectionError` derives from `OSError`,
    # so `pytest.raises(RuntimeError)` has already ruled it out and an
    # `isinstance` check here could never have failed whatever the bindings did.
    assert raised.type is RuntimeError


class _Impostor(http.server.BaseHTTPRequestHandler):
    """A server that answers, but not with anything the client can use.

    Every GET succeeds — 200, valid JSON, wrong shape. That is the whole trick:
    the transport is fine and the status is fine, so a client that cannot tell a
    decode failure from a network one has nothing left to blame but the network.
    """

    def do_GET(self):  # noqa: N802 — the name is BaseHTTPRequestHandler's
        body = b'{"not": "the shape you asked for"}'
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args):
        """Silent: the suite's output is not the place for request logs."""


def test_a_body_the_client_cannot_read_is_not_a_connection_problem():
    """The exact failure that hid in `clients()` for as long as it did.

    A shape mismatch means the two ends disagree about a version. Reporting it
    as `ConnectionError` sends a reader looking for a network fault that is not
    there — and, before this, made the fixture skip the tests that would have
    caught it.
    """
    from toboggan_py import Toboggan

    server = http.server.HTTPServer(("127.0.0.1", 0), _Impostor)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        with pytest.raises(RuntimeError) as raised:
            Toboggan("127.0.0.1", server.server_port)
    finally:
        server.shutdown()
        server.server_close()

    assert raised.type is RuntimeError, "a decode failure is not a network fault"
    assert "could not be read" in str(raised.value)
