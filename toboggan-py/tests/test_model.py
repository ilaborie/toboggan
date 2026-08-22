"""The values the getters hand back, and the shapes the docs promise.

Most of this surface had no test at all. Two parts of it are the kind that break
silently: `Slide.kind` is a fixed set of strings the stub names, and the parallel
arrays on `Talk` are documented to line up with `titles` by index. Neither has a
compiler behind it on the Python side, and a mismatch shows up as wrong output
rather than as an error.
"""

import pytest

KINDS = {"cover", "part", "standard"}


def test_every_slide_reports_a_kind_the_stub_names(presenter):
    """`kind` is a closed set, and it comes from a Rust enum.

    It used to be `format!("{:?}")` on that enum, so renaming a variant changed
    the Python API with nothing in the workspace to notice. It goes through a
    total match now; this is the other end of that.
    """
    kinds = {slide.kind for slide in presenter.slides}
    assert kinds <= KINDS, f"unexpected slide kinds: {sorted(kinds - KINDS)}"
    assert "cover" in kinds, "the guide deck opens with a cover slide"


def test_a_slide_reports_its_text_and_planned_time(presenter):
    slide = presenter.slides[0]

    assert isinstance(slide.title, str)
    assert isinstance(slide.body, str)
    assert isinstance(slide.notes, str)
    assert slide.duration is None or isinstance(slide.duration, float)

    # Never "web": the server filters web-hidden slides out before they reach a
    # client, so a slide that is here is a slide that is visible on the web.
    assert "web" not in slide.hidden_in


def test_the_talks_per_slide_arrays_line_up_with_its_titles(presenter):
    """The length relation the docs promise, which nothing enforced.

    The wire form is *either* empty *or* one per slide, and empty means "not
    computed" rather than "none" — so `zip(titles, durations)` used to yield
    nothing at all in that case, silently. They are padded now, which makes the
    relation unconditional and this assertion meaningful.
    """
    talk = presenter.talk
    slides = len(presenter.slides)

    assert len(talk.titles) == slides
    assert len(talk.step_counts) == slides
    assert len(talk.durations) == slides

    # The documented use, which must never be silently empty.
    assert len(list(zip(talk.titles, talk.step_counts, talk.durations))) == slides


def test_the_talk_reports_its_metadata(presenter):
    talk = presenter.talk

    assert talk.title
    # `YYYY-MM-DD`, as the stub promises — the format comes from a `Display`
    # impl three crates away, so it is worth pinning here.
    assert len(talk.date) == 10 and talk.date[4] == talk.date[7] == "-"
    assert talk.lang is None or isinstance(talk.lang, str)
    assert talk.footer is None or isinstance(talk.footer, str)
    assert talk.head is None or isinstance(talk.head, str)


def test_out_of_range_indexing_raises_but_get_returns_none(presenter):
    """The paired total/partial accessors, and the exception type the stub names."""
    slides = presenter.slides
    count = len(slides)

    with pytest.raises(IndexError):
        slides[count]
    with pytest.raises(IndexError):
        slides[-count - 1]

    assert slides.get(count) is None
    assert slides.get(-count - 1) is None

    # Negative indices count from the end, as they do for any sequence. This
    # used to raise `OverflowError` from the argument conversion.
    assert slides[-1].title == slides[count - 1].title


def test_the_three_states_are_exactly_one(presenter):
    """`kind` and the three booleans have to agree, whichever the deck is in."""
    presenter.first()
    state = presenter.state
    assert (state.is_init, state.is_running, state.is_done).count(True) == 1
    assert state.kind == "running"
    assert state.is_first_slide is True
    assert state.slide == 1
    assert state.step == 0

    presenter.last()
    state = presenter.state
    assert state.is_last_slide is True
    assert state.total_slides == len(presenter.slides)

    # Past the last slide is `Done` rather than an error — the deck runs out
    # rather than refusing to move.
    presenter.next()
    state = presenter.state
    assert state.kind == "done"
    assert state.is_done is True
    assert (state.is_init, state.is_running, state.is_done).count(True) == 1
