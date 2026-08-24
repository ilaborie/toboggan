"""Every documented call, written the way the docs write it, and type-checked.

`test_stubs.py` compares *names* in both directions, which catches the most
damaging kind of drift and none of the rest: it never reads an annotation, so
`def clients(self) -> None` would sail through it. This file is the other half.
Nothing here runs — mypy is the whole test — so it needs no server and asserts
nothing at runtime.

It earns its place: `for slide in client.slides` was an error against the stub
while working perfectly at runtime, because pyo3's legacy sequence protocol is
invisible to a type checker. Nothing else in the suite could have found that.

Add to it whenever the stub promises something new. A promise no caller has
written down is a promise nobody has checked.
"""

from typing import List, Optional

from toboggan_py import ClientInfo, Slide, Slides, State, Talk, Toboggan


def metadata(client: Toboggan) -> None:
    talk: Talk = client.talk
    title: str = talk.title
    date: str = talk.date
    footer: Optional[str] = talk.footer
    head: Optional[str] = talk.head
    lang: Optional[str] = talk.lang
    titles: List[str] = talk.titles

    # Both promised to be exactly as long as `titles`, so zipping them is the
    # documented use and must type-check as one.
    steps: List[int] = talk.step_counts
    durations: List[Optional[float]] = talk.durations
    for slide_title, step_count, planned in zip(titles, steps, durations):
        print(slide_title, step_count, planned)

    print(title, date, footer, head, lang)


def deck(client: Toboggan) -> None:
    slides: Slides = client.slides
    count: int = len(slides)

    # Iteration: the promise that could not be kept in the stub alone.
    for slide in slides:
        kind: str = slide.kind
        body: str = slide.body
        notes: str = slide.notes
        duration: Optional[float] = slide.duration
        hidden: List[str] = slide.hidden_in
        print(kind, body, notes, duration, hidden)

    first: Slide = slides[0]
    last: Slide = slides[-1]
    maybe: Optional[Slide] = slides.get(count - 1)
    missing: Optional[Slide] = slides.get(9999)
    print(first.title, last.title, maybe, missing)


def position(client: Toboggan) -> None:
    state: State = client.state

    # Zero-argument, because the state knows the deck it belongs to.
    on_first: bool = state.is_first_slide
    on_last: bool = state.is_last_slide
    total: int = state.total_slides

    slide: Optional[int] = state.slide
    step: Optional[int] = state.step

    # The closed set a checker can narrow on.
    if state.kind == "running":
        print("running", slide, step)
    elif state.kind == "done":
        print("done")

    print(state.is_init, state.is_running, state.is_done, on_first, on_last, total)


def room(client: Toboggan) -> None:
    people: List[ClientInfo] = client.clients()
    for person in people:
        name: str = person.name
        address: str = person.ip_addr
        connected: str = person.connected_at
        presenting: bool = person.is_presenter
        print(name, address, connected, presenting, person.role)


def drive(client: Toboggan) -> None:
    client.first()
    client.next()
    client.next_step()
    client.previous_step()
    client.previous()
    client.goto(2)
    client.last()
    client.blink()


def session() -> None:
    # The context manager, and the keyword arguments the constructor documents.
    with Toboggan(host="localhost", port=8080, presenter_token=None) as client:
        role: Optional[str] = client.role
        presenting: bool = client.is_presenter
        print(role, presenting)

        metadata(client)
        deck(client)
        position(client)
        room(client)
        drive(client)

    # Also usable without the `with`.
    other = Toboggan()
    other.close()
