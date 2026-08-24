"""What `close()` promises, and what dropping a client does instead.

`close()` is the documented way to put a client away: it releases the GIL and
waits for the runtime's threads, so the interpreter is never frozen inside a
shutdown nobody asked for. Nothing checked that it worked — the whole contract
lived in a docstring, and the suite itself never called it.

The last test here is the one that matters most and asserts least: a client left
to the garbage collector must not wedge the interpreter. That path runs
`Runtime::drop` with the GIL held, and with `pyo3_log` in the picture the worker
threads reach for the same GIL on their way out. `Drop` hands the runtime to
`shutdown_background` for exactly that reason.
"""

import subprocess
import sys
import textwrap

import pytest

from toboggan_py import Toboggan


def test_close_makes_every_server_call_say_so(server):
    host, port = server
    client = Toboggan(host, port)
    client.close()

    # The message names the remedy, because there is no reopening a client.
    for call in (lambda: client.talk, lambda: client.slides, lambda: client.next()):
        with pytest.raises(RuntimeError, match="closed"):
            call()


def test_close_is_idempotent(server):
    """Closing twice is not an error; a `with` around an explicit close does it."""
    host, port = server
    client = Toboggan(host, port)
    client.close()
    client.close()


def test_with_closes_on_the_way_out(server):
    host, port = server
    with Toboggan(host, port) as client:
        assert client.talk.title

    with pytest.raises(RuntimeError, match="closed"):
        _ = client.talk


def test_with_closes_when_the_block_raises(server):
    """`__exit__` returns False, so the exception is not swallowed on the way."""
    host, port = server
    sentinel = RuntimeError("from inside the block")

    with pytest.raises(RuntimeError) as raised:
        with Toboggan(host, port) as client:
            raise sentinel

    assert raised.value is sentinel
    with pytest.raises(RuntimeError, match="closed"):
        _ = client.talk


# Its own interpreter: the failure this guards is a deadlock between a worker
# thread and the collector, which wedges the process rather than raising. A
# pytest timeout cannot help with the GIL held, so the only way to get a
# readable failure instead of a hung job is to watch it from outside.
def test_a_collected_client_does_not_wedge_the_interpreter(server):
    host, port = server
    probe = textwrap.dedent(f"""
        import gc

        from toboggan_py import Toboggan

        # Dropped without close(), which is what a script that forgets does.
        for _ in range(5):
            client = Toboggan({host!r}, {port})
            assert client.talk.title
            del client
            gc.collect()

        print("survived")
    """)

    finished = subprocess.run(
        [sys.executable, "-c", probe],
        capture_output=True,
        text=True,
        timeout=120,
    )

    assert finished.returncode == 0, finished.stderr
    assert "survived" in finished.stdout
