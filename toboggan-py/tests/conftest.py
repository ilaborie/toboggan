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
import sys
import time
import urllib.error
import urllib.request

import pytest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WORKSPACE_ROOT = os.path.dirname(REPO_ROOT)

# 51 `<!-- pause -->` markers across 44 slides. `examples/riir-folder` has none
# at all, which would make every step assertion vacuous, and
# `examples/demo-terminal` spawns shell PTYs for its terminal slides.
DECK = os.path.join(WORKSPACE_ROOT, "examples", "toboggan-guide", "slides")

# Remote clients need this to present; a loopback client presents without it.
# One server offering it therefore covers all three role cases.
PRESENTER_TOKEN = "test-presenter-token"

# A cold `cargo run` builds the workspace first.
READY_TIMEOUT = 300.0


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

    manifest = os.path.join(WORKSPACE_ROOT, "Cargo.toml")
    return ["cargo", "run", "--manifest-path", manifest, "-p", "toboggan", "--", *common]


def _wait_until_healthy(process, port):
    """Poll `/health` until the server answers, as Playwright's webServer does."""
    url = f"http://127.0.0.1:{port}/health"
    deadline = time.monotonic() + READY_TIMEOUT

    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(
                f"the server exited with {process.returncode} before becoming ready"
            )
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if response.status == 200:
                    return
        except (urllib.error.URLError, ConnectionError, socket.timeout):
            time.sleep(0.1)

    raise RuntimeError(f"the server was not ready within {READY_TIMEOUT:.0f}s")


@pytest.fixture(scope="session")
def server():
    """Host and port of a running server, for the whole session.

    Session-scoped because starting one costs a process (and possibly a cargo
    build). The consequence is that the deck is shared mutable state across
    every test, so these tests must not run in parallel — the same constraint
    that makes the Playwright suite set `workers: 1`.
    """
    if not os.path.isdir(DECK):
        pytest.skip(f"the example deck is missing: {DECK}")
    if not os.environ.get("TOBOGGAN_BIN") and shutil.which("cargo") is None:
        pytest.skip("neither TOBOGGAN_BIN nor cargo is available to start a server")

    port = int(os.environ.get("TOBOGGAN_PY_TEST_PORT") or _free_port())
    process = subprocess.Popen(
        _server_command(port),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        cwd=WORKSPACE_ROOT,
    )

    try:
        _wait_until_healthy(process, port)
        yield "localhost", port
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()


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


requires_lan = pytest.mark.skipif(
    lan_address() is None,
    reason="no non-loopback address; an audience client cannot be created",
)


def remote_client(port, **kwargs):
    """A client connecting from this machine's own non-loopback address.

    Having such an address does not guarantee it is reachable from here — a
    container or a locked-down runner may route it nowhere. That is a fact about
    the host, not a fault in the bindings, so it skips rather than fails.
    """
    from toboggan_py import Toboggan

    address = lan_address()
    if address is None:
        pytest.skip("no non-loopback address available")

    try:
        return Toboggan(address, port, **kwargs)
    except ConnectionError as unreachable:
        pytest.skip(f"this host cannot reach itself at {address}: {unreachable}")
