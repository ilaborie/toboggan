"""A client never claims a role; it offers a token and the server decides.

The server grants the presenter role to a loopback connection unconditionally,
so an audience client cannot be made over `localhost` — every audience case here
connects to this machine's own non-loopback address instead.
"""

import pytest
from toboggan_py import Toboggan

from conftest import PRESENTER_TOKEN, remote_client, requires_lan


def test_a_local_client_presents(presenter):
    assert presenter.is_presenter is True
    assert presenter.role == "presenter"


def test_a_local_client_presents_without_a_token(server):
    """The token is for clients that are not on this machine."""
    host, port = server
    assert Toboggan(host, port, presenter_token=None).is_presenter is True


@requires_lan
def test_a_remote_client_without_a_token_is_audience(server):
    _, port = server
    assert remote_client(port).is_presenter is False


@requires_lan
def test_a_remote_client_with_the_token_presents(server):
    _, port = server
    remote = remote_client(port, presenter_token=PRESENTER_TOKEN)
    assert remote.is_presenter is True


@requires_lan
def test_a_remote_presenter_drives_the_deck_synchronously(server, presenter):
    _, port = server
    remote = remote_client(port, presenter_token=PRESENTER_TOKEN)

    remote.goto(2)
    assert remote.state.slide == 2

    remote.next()
    assert remote.state.slide == 3


@requires_lan
def test_an_audience_client_is_refused_loudly(server, presenter):
    """The refusal must raise. It used to return successfully and move nothing,
    which is the worst of both: a script reported success over a deck that had
    not budged."""
    _, port = server
    audience = remote_client(port)

    presenter.goto(4)
    before = presenter.state.slide

    for attempt in (audience.next, audience.previous, audience.first, audience.last):
        with pytest.raises(PermissionError):
            attempt()

    with pytest.raises(PermissionError):
        audience.goto(7)

    assert presenter.state.slide == before, "a refused command moved the deck"


@requires_lan
def test_an_audience_client_cannot_enumerate_the_room(server):
    _, port = server
    with pytest.raises(PermissionError):
        remote_client(port).clients()


def test_a_presenter_can_enumerate_the_room(presenter):
    names = [client.name for client in presenter.clients()]
    assert "Python" in names

    for client in presenter.clients():
        assert client.role in ("presenter", "audience")
        assert client.is_presenter == (client.role == "presenter")
        assert client.ip_addr


@requires_lan
@pytest.mark.parametrize("blank", ["", "   ", "\t\n"])
def test_a_blank_token_is_no_token(server, blank):
    """`Secret::new` decides what counts as a token, on both sides of the wire.
    A blank one is no token rather than one the server can only refuse."""
    _, port = server
    assert remote_client(port, presenter_token=blank).is_presenter is False


@requires_lan
def test_the_token_falls_back_to_the_environment(server, monkeypatch):
    """`toboggan tui` and `toboggan desktop` read the same variable, so a script
    run beside them should not have to be told separately."""
    _, port = server
    monkeypatch.setenv("TOBOGGAN_PRESENTER_TOKEN", PRESENTER_TOKEN)
    assert remote_client(port).is_presenter is True


@requires_lan
def test_an_explicit_token_beats_the_environment(server, monkeypatch):
    _, port = server
    monkeypatch.setenv("TOBOGGAN_PRESENTER_TOKEN", PRESENTER_TOKEN)
    assert remote_client(port, presenter_token="wrong").is_presenter is False
