"""A command the server rejects raises, rather than quietly doing nothing."""

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

    assert not isinstance(raised.value, ConnectionError)
