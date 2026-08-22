"""Navigation is synchronous: when a call returns, the deck has moved.

This is the regression guard for the defect these tests were written for. Before
the fix, `next()` pushed its command onto a channel and returned, and the state
arrived a socket round trip later — so every assertion in this file failed, and
`example.py` needed a `sleep(1)` after each command to look correct.

Nothing here sleeps. That is the whole point: if a sleep is ever needed to make
this file pass, the leak is back.
"""


def test_next_lands_before_the_state_is_read(presenter):
    total = len(presenter.slides)
    presenter.first()

    for expected in range(2, min(total, 20) + 1):
        presenter.next()
        assert presenter.state.slide == expected


def test_goto_lands_before_the_state_is_read(presenter):
    total = len(presenter.slides)

    for target in (3, 1, total, 2, total // 2):
        presenter.goto(target)
        assert presenter.state.slide == target


def test_previous_lands_before_the_state_is_read(presenter):
    presenter.goto(5)

    for expected in (4, 3, 2, 1):
        presenter.previous()
        assert presenter.state.slide == expected


def test_first_and_last_land_before_the_state_is_read(presenter):
    total = len(presenter.slides)

    presenter.goto(4)
    presenter.first()
    assert presenter.state.slide == 1

    presenter.last()
    assert presenter.state.slide == total


def test_next_step_lands_before_the_state_is_read(presenter):
    """A slide with reveals, so this tests steps rather than slide changes."""
    counts = presenter.talk.step_counts
    with_steps = next((i + 1 for i, count in enumerate(counts) if count > 1), None)
    assert with_steps is not None, f"the deck has no slide with reveals: {counts[:10]}"

    presenter.goto(with_steps)
    before = presenter.state.step

    presenter.next_step()
    assert presenter.state.step == before + 1

    presenter.previous_step()
    assert presenter.state.step == before


def test_a_long_run_never_reads_a_stale_state(presenter):
    """Sustained, mixed traffic — a race that shows up one time in fifty is
    still a race, and a handful of calls would not find it."""
    total = len(presenter.slides)
    stale = []

    for index in range(120):
        target = (index % total) + 1
        presenter.goto(target)
        observed = presenter.state.slide
        if observed != target:
            stale.append((index, target, observed))

    assert not stale, f"{len(stale)} stale reads, first few: {stale[:5]}"
