"""A live Toboggan server for the binding tests.

The bindings are a client. Nothing about them can be tested honestly without
something to talk to, and the interesting behaviour — a command that has landed
by the time the call returns, a refusal that raises — lives in the round trip.
So these tests drive the real binary against a real deck, the same way the web
client's Playwright suite does (`toboggan-web/playwright.config.ts`).
"""

import os
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

import pytest

# `toboggan-py/`, then the repository above it. Named the way the rest of the
# repo uses the words: this crate is deliberately *outside* the cargo workspace,
# so the two are not the same directory and the distinction matters.
CRATE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REPO_ROOT = os.path.dirname(CRATE_ROOT)

# Chosen for its `<!-- pause -->` markers, which the step assertions need:
# `examples/riir-folder` has none at all, which would make them vacuous, and
# `examples/demo-terminal` spawns shell PTYs for its terminal slides.
DECK = os.path.join(REPO_ROOT, "examples", "toboggan-guide", "slides")

# Remote clients need this to present; a loopback client presents without it.
# One server offering it therefore covers all three role cases.
PRESENTER_TOKEN = "test-presenter-token"

# A cold `cargo run` builds the workspace first.
READY_TIMEOUT = 300.0


def _skip_or_fail(reason):
    """Skip locally, fail where the precondition was promised.

    pytest exits 0 when every test skipped, so a session-scoped skip is
    indistinguishable from a suite that passed. That is tolerable on a laptop
    missing a prerequisite and not tolerable in CI, which controls its own
    environment and is the only place anyone is watching. `TOBOGGAN_PY_STRICT`
    is what CI sets to say "these preconditions are my job, so their absence is
    a failure, not a fact of life".
    """
    if os.environ.get("TOBOGGAN_PY_STRICT"):
        pytest.fail(f"{reason} (TOBOGGAN_PY_STRICT is set)")
    pytest.skip(reason)


def _free_port():
    """A port nobody is on, asked of the OS rather than guessed.

    8080 (the server default, and `mise serve`), 8137 (Playwright) and 8000
    (vite) are all spoken for, and a developer running any of them while the
    tests run would otherwise get a collision that reads as a test failure.
    """
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def _server_command(port):
    """How to start the server, preferring a binary someone already built.

    `.cargo/config.toml` pins `[build] target = "aarch64-apple-darwin"`, so the
    debug binary is *not* at `target/debug/` on a developer's machine. That is
    why the path arrives in an environment variable rather than being guessed —
    the same reason `playwright.config.ts` takes `TOBOGGAN_BIN`.
    """
    common = [
        "-p", DECK,
        "--host", "0.0.0.0",
        "--port", str(port),
        "--presenter-token", PRESENTER_TOKEN,
    ]

    binary = os.environ.get("TOBOGGAN_BIN")
    if binary:
        return [binary, *common]

    manifest = os.path.join(REPO_ROOT, "Cargo.toml")
    return ["cargo", "run", "--manifest-path", manifest, "-p", "toboggan", "--", *common]


def _wait_until_healthy(process, port, log):
    """Poll `/health` until the server answers, as Playwright's webServer does.

    Carries `log` so that a failure says *why*. A parse error in the deck, a port
    already taken, a panic — all of it goes to the server's stderr, and an exit
    code on its own is a dead end on a CI runner nobody can reproduce.
    """
    url = f"http://127.0.0.1:{port}/health"
    deadline = time.monotonic() + READY_TIMEOUT
    last_answer = "nothing yet"

    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(
                f"the server exited with {process.returncode} before becoming "
                f"ready.\n{_tail(log)}"
            )
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if response.status == 200:
                    return
                last_answer = f"HTTP {response.status}"
        except (urllib.error.URLError, ConnectionError, socket.timeout) as unready:
            # `HTTPError` is a `URLError`, so a server answering 500 on every
            # poll lands here too — hence recording what it said rather than
            # reporting only that the deadline passed.
            last_answer = f"{type(unready).__name__}: {unready}"

        # At the bottom of the loop, not inside the `except`: a non-200 answer
        # used to fall straight through and spin a tight five-minute request
        # loop against the server it was waiting for.
        time.sleep(0.1)

    raise RuntimeError(
        f"the server was not ready within {READY_TIMEOUT:.0f}s "
        f"(last answer: {last_answer}).\n{_tail(log)}"
    )


def _tail(log, limit=4000):
    """The end of the server's output, for an error message."""
    log.flush()
    with open(log.name, encoding="utf-8", errors="replace") as written:
        output = written.read()[-limit:].strip()
    return f"--- server output ---\n{output}" if output else "(the server said nothing)"


@pytest.fixture(scope="session")
def server():
    """Host and port of a running server, for the whole session.

    Session-scoped because starting one costs a process (and possibly a cargo
    build). The consequence is that the deck is shared mutable state across
    every test, so these tests must not run in parallel — the same constraint
    that makes the Playwright suite set `workers: 1`.
    """
    if not os.path.isdir(DECK):
        _skip_or_fail(f"the example deck is missing: {DECK}")
    if not os.environ.get("TOBOGGAN_BIN") and shutil.which("cargo") is None:
        _skip_or_fail("neither TOBOGGAN_BIN nor cargo is available to start a server")

    port = int(os.environ.get("TOBOGGAN_PY_TEST_PORT") or _free_port())

    # Captured, not discarded. This is the one place that knows why a server
    # failed to start, and `DEVNULL` left a bare exit code to debug from.
    with tempfile.NamedTemporaryFile(
        mode="w+", suffix=".log", prefix="toboggan-server-", delete=False
    ) as log:
        process = subprocess.Popen(
            _server_command(port),
            stdout=log,
            stderr=subprocess.STDOUT,
            cwd=REPO_ROOT,
        )

        try:
            _wait_until_healthy(process, port, log)
            yield "localhost", port
        finally:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                # Reaped, not just signalled: `kill` alone leaves a zombie, in
                # the `finally` that exists to guarantee cleanup.
                process.wait(timeout=10)
            os.unlink(log.name)


@pytest.fixture
def presenter(server):
    """A client on the server's own machine, which always presents."""
    from toboggan_py import Toboggan

    host, port = server
    return Toboggan(host, port)


def lan_address():
    """This machine's own non-loopback address, or None if it has none.

    The role tests need one: the server grants the presenter role to loopback
    unconditionally, so an audience client cannot be made over `localhost` at
    all — it has to arrive from somewhere else, even if that somewhere else is
    this same machine under a different address.
    """
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as probe:
            # Never actually sends: connect() on UDP just picks the route, which
            # is what tells us which local address the outside world would see.
            probe.connect(("192.0.2.1", 53))
            address = probe.getsockname()[0]
    except OSError:
        return None

    return None if address.startswith("127.") else address


# The role tests are the only coverage of the token-on-REST fix and of the
# 403 → PermissionError mapping. Skipping them costs the whole security surface
# of these bindings, so under TOBOGGAN_PY_STRICT a runner without a LAN address
# is a broken runner rather than a reason to report success over nine absent
# tests.
requires_lan = pytest.mark.skipif(
    lan_address() is None and not os.environ.get("TOBOGGAN_PY_STRICT"),
    reason="no non-loopback address; an audience client cannot be created",
)


def remote_client(port, **kwargs):
    """A client connecting from this machine's own non-loopback address.

    Having such an address does not guarantee it is reachable from here — a
    container or a locked-down runner may route it nowhere. That is a fact about
    the host, not a fault in the bindings, so it skips rather than fails.

    Only that one case skips. Catching `ConnectionError` wholesale used to turn
    *any* constructor failure into a green skip, and since every API failure was
    a `ConnectionError` back then, that quietly covered a 403, a 500 and a body
    the client could not parse — the whole role suite could vanish while
    reporting success. Anything that is not a routing problem is re-raised.
    """
    from toboggan_py import Toboggan

    address = lan_address()
    if address is None:
        _skip_or_fail("no non-loopback address available")

    try:
        return Toboggan(address, port, **kwargs)
    except ConnectionError as unreachable:
        if not _is_unroutable(unreachable):
            raise
        _skip_or_fail(f"this host cannot reach itself at {address}: {unreachable}")


# What the OS says when a packet has nowhere to go. A refusal is *not* here on
# purpose: a server that refuses is a server that answered, and against a port
# we know is listening that means the bindings are at fault, not the network.
_UNROUTABLE = ("no route to host", "network is unreachable", "host is unreachable")


def _is_unroutable(error):
    return any(phrase in str(error).lower() for phrase in _UNROUTABLE)
