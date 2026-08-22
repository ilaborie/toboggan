"""Toboggan for Python.

This module provides Python bindings for the Toboggan presentation system,
enabling real-time multi-client synchronization: commands travel over REST and
answer their caller, while state and deck changes are pushed over a WebSocket.
"""

from types import TracebackType
from typing import Iterator, List, Literal, Optional, Type, final

__all__ = ["ClientInfo", "Slide", "Slides", "State", "Talk", "Toboggan"]

@final
class Talk:
    """Presentation metadata.

    Everything about a deck except its slides: title, date, language, optional
    footer and `<head>` markup, and the title, step count and planned duration
    of every slide.

    Note:
        This class cannot be instantiated directly. Obtain instances
        via the `Toboggan.talk` property.
    """

    @property
    def title(self) -> str:
        """The presentation title."""
        ...

    @property
    def date(self) -> str:
        """The date the talk is given, as `YYYY-MM-DD`."""
        ...

    @property
    def footer(self) -> Optional[str]:
        """Markup shown at the foot of every slide, if the deck has any."""
        ...

    @property
    def head(self) -> Optional[str]:
        """Markup injected into `<head>` — fonts, custom CSS — if any."""
        ...

    @property
    def lang(self) -> Optional[str]:
        """The deck's BCP 47 language tag, e.g. `"fr"`, or None if unset."""
        ...

    @property
    def titles(self) -> List[str]:
        """Every slide's title, in presentation order."""
        ...

    @property
    def step_counts(self) -> List[int]:
        """Number of reveals per slide, for showing step progress.

        Always exactly as long as `titles`, read against it by index. A deck
        whose steps the server did not compute reports 0 for every slide rather
        than an empty list, so `zip(talk.titles, talk.step_counts)` is never
        silently empty.
        """
        ...

    @property
    def durations(self) -> List[Optional[float]]:
        """Planned speaking time per slide, in seconds.

        From each slide's `duration` front matter, and None where the author
        did not declare one. Always exactly as long as `titles`, and in the same
        units and type as `Slide.duration`.
        """
        ...

    def __repr__(self) -> str:
        """Returns a detailed string representation of the talk metadata."""
        ...

    def __str__(self) -> str:
        """Returns the presentation title."""
        ...

@final
class Slide:
    """A single slide in the presentation.

    Note:
        This class cannot be instantiated directly. Obtain instances by
        indexing `Toboggan.slides`.
    """

    @property
    def kind(self) -> Literal["cover", "part", "standard"]:
        """`"cover"`, `"part"` for a section title, or `"standard"`."""
        ...

    @property
    def title(self) -> str:
        """The slide's heading, as words rather than markup.

        Empty for a slide that deliberately has none.
        """
        ...

    @property
    def body(self) -> str:
        """Everything below the heading."""
        ...

    @property
    def notes(self) -> str:
        """Speaker notes — never shown on the projector."""
        ...

    @property
    def duration(self) -> Optional[float]:
        """Speaking time the author planned for this slide, in seconds.

        None where the front matter did not declare one.
        """
        ...

    @property
    def hidden_in(self) -> List[str]:
        """Render targets this slide is excluded from — in practice only
        `"pdf"`.

        The server never sends web-hidden slides to a client at all, so a slide
        you can see here is a slide that is in the web deck; `"web"` cannot
        appear. Empty therefore means the slide is in the PDF too.
        """
        ...

    def __repr__(self) -> str:
        """Returns the slide's kind and title."""
        ...

@final
class Slides:
    """Collection of slides in the presentation.

    Supports `len()`, indexing, and iteration.

    Note:
        This class cannot be instantiated directly. Obtain instances
        via the `Toboggan.slides` property.
    """

    def get(self, index: int) -> Optional[Slide]:
        """Returns the slide at `index`, or None if out of range.

        Negative indices count from the end, as they do for any sequence.
        """
        ...

    def __len__(self) -> int:
        """Returns the number of slides."""
        ...

    def __getitem__(self, index: int, /) -> Slide:
        """Returns the slide at `index`.

        Negative indices count from the end, so `slides[-1]` is the last slide.

        Raises:
            IndexError: If `index` is out of range.
        """
        ...

    def __iter__(self) -> Iterator[Slide]:
        """Iterates the slides in presentation order."""
        ...

    def __repr__(self) -> str:
        """Returns a numbered list of slide titles."""
        ...

    def __str__(self) -> str:
        """Returns a numbered list of slide titles."""
        ...

@final
class State:
    """Current presentation state.

    Represents the real-time state of the presentation, synchronized across
    all connected clients. There are exactly three states, and no pause — a
    deck that is not moving is a running deck nobody is sending commands about:

    - Init: nothing has been shown yet
    - Running: showing a slide
    - Done: past the last slide

    Note:
        This class cannot be instantiated directly. Obtain instances
        via the `Toboggan.state` property.
    """

    @property
    def is_init(self) -> bool:
        """Whether nothing has been shown yet."""
        ...

    @property
    def is_running(self) -> bool:
        """Whether a slide is currently showing."""
        ...

    @property
    def is_done(self) -> bool:
        """Whether the deck is past its last slide."""
        ...

    @property
    def slide(self) -> Optional[int]:
        """The current slide number, counting from 1, or None before the
        deck has started."""
        ...

    @property
    def step(self) -> Optional[int]:
        """The reveal currently showing, counting from 0, or None before the
        deck has started."""
        ...

    @property
    def kind(self) -> Literal["init", "running", "done"]:
        """Which of the three states this is.

        The same question as `is_init`/`is_running`/`is_done`, in a form a type
        checker can narrow on and a `match` can dispatch over.
        """
        ...

    @property
    def total_slides(self) -> int:
        """How many slides the deck had when this state was read."""
        ...

    @property
    def is_first_slide(self) -> bool:
        """Whether the deck is on its first slide.

        An empty deck is on neither its first nor its last slide.
        """
        ...

    @property
    def is_last_slide(self) -> bool:
        """Whether the deck is on its last slide."""
        ...

    def __repr__(self) -> str:
        """Returns a detailed string representation of the current state."""
        ...

    def __str__(self) -> str:
        """Returns a detailed string representation of the current state."""
        ...

@final
class ClientInfo:
    """A snapshot of one client connected to the server.

    Note:
        This class cannot be instantiated directly. Obtain instances
        via `Toboggan.clients()`.
    """

    @property
    def name(self) -> str:
        """The name the client registered under — `"tui"`, `"Python"`, …"""
        ...

    @property
    def ip_addr(self) -> str:
        """Where the connection came from.

        Not on its own what decided the role: loopback presents unconditionally,
        but a connection from anywhere else presents too if it carried the
        token.
        """
        ...

    @property
    def role(self) -> Literal["presenter", "audience"]:
        """What the server granted this client: `"presenter"` or `"audience"`."""
        ...

    @property
    def is_presenter(self) -> bool:
        """Whether this client may drive the deck and open terminals."""
        ...

    @property
    def connected_at(self) -> str:
        """When the client registered, as an RFC 3339 timestamp."""
        ...

    def __repr__(self) -> str:
        """Returns the client's name, role, address and connection time."""
        ...

@final
class Toboggan:
    """Toboggan presentation client.

    Main client for connecting to a Toboggan presentation server.
    Manages WebSocket communication, state synchronization, and provides
    methods for controlling the presentation (navigation and step reveals).

    The client automatically maintains a persistent connection to the server
    and synchronizes state changes across all connected clients in real-time.

    Navigation is synchronous: `next()` and its siblings return only once the
    server has applied the command, so reading `state` straight afterwards
    reports where the deck now is. No sleep, no polling. The socket stays for
    the job only it can do — reporting moves *other* clients made, and deck
    reloads.

    Roles:
        A client never claims a role — it offers a presenter token and the
        server decides. A connection from the machine running the server
        presents; a connection from anywhere else presents only if it carries
        the token. An audience client's navigation raises `PermissionError`
        rather than quietly doing nothing; `is_presenter` says so in advance.

    Example:
        ```python
        from toboggan_py import Toboggan

        # Connect to server
        client = Toboggan("localhost", 8080)

        # Access presentation metadata
        print(client.talk)
        print(client.slides)

        # Navigate slides — each returns once the deck has moved
        client.next()
        client.previous()
        client.goto(12)

        # Correct immediately: no sleep needed
        print(client.state)
        ```
    """

    def __init__(
        self,
        host: str = "localhost",
        port: int = 8080,
        presenter_token: Optional[str] = None,
    ) -> None:
        """Creates a new Toboggan client and connects to the server.

        Args:
            host: Server hostname or IP address (default: "localhost")
            port: Server port number (default: 8080)
            presenter_token: Token offered at registration, needed only when
                the server runs on another machine — a client on the server's
                own machine always presents. Falls back to the
                `TOBOGGAN_PRESENTER_TOKEN` environment variable, which is what
                the other Toboggan clients read too. A blank token is treated
                as no token.

        Raises:
            ConnectionError: If the server cannot be reached.
            RuntimeError: If the server refuses, or answers something this
                client cannot read — which usually means the two ends are
                different versions.
            PermissionError: If the server answers 403.
            OSError: If the async runtime cannot be started.
            OverflowError: If `port` is outside `0..=65535`.

        A registration the server has not yet answered is not an error — `role`
        reports None until it arrives. Nor is a socket that has not come up yet;
        the client keeps trying in the background.
        """
        ...

    @property
    def talk(self) -> Talk:
        """Presentation metadata.

        Returns information about the presentation including title, date,
        language, footer content, and all slide titles.

        Raises:
            RuntimeError: If the deck was reloaded and could not be refetched,
                so what is cached here is the deck as it was before.
        """
        ...

    @property
    def slides(self) -> Slides:
        """All slides in the presentation.

        Returns the complete collection of slides with their content,
        metadata, and ordering.

        Raises:
            RuntimeError: If the deck was reloaded and could not be refetched,
                so what is cached here is the deck as it was before.
        """
        ...

    @property
    def state(self) -> State:
        """Current presentation state.

        Returns the real-time synchronized state showing which slide
        is currently displayed and whether the presentation is running,
        done, or in its initial state.

        This property reflects the state synchronized across all connected
        clients. Changes made by any client will be reflected here.

        Trustworthy immediately after a navigation call: those return only
        once the server has applied the command and this cache holds its
        answer.

        Raises:
            RuntimeError: If the deck was reloaded and could not be refetched,
                so what is cached here is the deck as it was before.
        """
        ...

    @property
    def role(self) -> Optional[Literal["presenter", "audience"]]:
        """The role the server granted this connection.

        `"presenter"`, `"audience"`, or None while registration is still
        unanswered. Re-reported after a reconnect, so a server restarted with
        a different token demotes this client here too.
        """
        ...

    @property
    def is_presenter(self) -> bool:
        """Whether this client may drive the deck.

        False for a connection from another machine that offered no presenter
        token: the navigation methods are then refused by the server.
        """
        ...

    def previous(self) -> None:
        """Navigates to the previous slide.

        Returns once the server has applied the move, so reading `state`
        straight afterwards reports where the deck now is.

        Raises:
            PermissionError: If this client is watching rather than presenting.
            RuntimeError: If the server rejects the command.
            ConnectionError: If the server cannot be reached.
        """
        ...

    def next(self) -> None:
        """Navigates to the next slide, skipping any reveals left on this one.

        Returns once the server has applied the move, so reading `state`
        straight afterwards reports the slide this call landed on.

        Raises:
            PermissionError: If this client is watching rather than presenting.
            RuntimeError: If the server rejects the command.
            ConnectionError: If the server cannot be reached.
        """
        ...

    def first(self) -> None:
        """Navigates to the first slide.

        Returns once the server has applied the move, so reading `state`
        straight afterwards reports where the deck now is.

        Raises:
            PermissionError: If this client is watching rather than presenting.
            RuntimeError: If the server rejects the command.
            ConnectionError: If the server cannot be reached.
        """
        ...

    def last(self) -> None:
        """Navigates to the last slide.

        Returns once the server has applied the move, so reading `state`
        straight afterwards reports where the deck now is.

        Raises:
            PermissionError: If this client is watching rather than presenting.
            RuntimeError: If the server rejects the command.
            ConnectionError: If the server cannot be reached.
        """
        ...

    def goto(self, slide: int) -> None:
        """Navigates to a specific slide.

        Args:
            slide: The slide number as printed on the slide, counting from 1.

        Returns once the server has applied the move, so reading `state`
        straight afterwards reports where the deck now is.

        Raises:
            PermissionError: If this client is watching rather than presenting.
            ValueError: If `slide` is 0. Slide numbers count from 1, and 0 used
                to land on slide 1 rather than say so.
            OverflowError: If `slide` is negative.
            RuntimeError: If the deck has no such slide. A number out of range
                is an error rather than a silent no-op.
            ConnectionError: If the server cannot be reached.
        """
        ...

    def next_step(self) -> None:
        """Reveals the next step, moving to the next slide once this one runs out.

        This is what a presenter remote and the space bar send.

        Returns once the server has applied the move, so reading `state`
        straight afterwards reports where the deck now is.

        Raises:
            PermissionError: If this client is watching rather than presenting.
            RuntimeError: If the server rejects the command.
            ConnectionError: If the server cannot be reached.
        """
        ...

    def previous_step(self) -> None:
        """Goes back one step, moving to the previous slide once this one runs out.

        Returns once the server has applied the move, so reading `state`
        straight afterwards reports where the deck now is.

        Raises:
            PermissionError: If this client is watching rather than presenting.
            RuntimeError: If the server rejects the command.
            ConnectionError: If the server cannot be reached.
        """
        ...

    def blink(self) -> None:
        """Flashes every other client, to get the room's attention.

        A blink moves nothing, so it leaves `state` alone.

        Raises:
            PermissionError: If this client is watching rather than presenting.
            RuntimeError: If the server rejects the command.
            ConnectionError: If the server cannot be reached.
        """
        ...

    def clients(self) -> List[ClientInfo]:
        """Lists who is currently connected.

        Presenter-only on the server — it reports names, roles and IP
        addresses — so an audience client cannot enumerate the room.

        Raises:
            PermissionError: If this client is watching rather than presenting.
            ConnectionError: If the server cannot be reached.
        """
        ...

    def close(self) -> None:
        """Disconnects and shuts this client's runtime down.

        Idempotent. Every other call raises `RuntimeError` afterwards.

        Worth calling rather than leaving to the garbage collector: dropping a
        client shuts down a multi-threaded runtime, and the collector does that
        while holding the GIL — so the interpreter can sit frozen in a shutdown
        nobody asked for. Using the client as a context manager is the easy way.
        """
        ...

    def __enter__(self) -> "Toboggan":
        """Returns the client itself, for use in a `with` block."""
        ...

    def __exit__(
        self,
        exc_type: Optional[Type[BaseException]],
        exc_value: Optional[BaseException],
        traceback: Optional[TracebackType],
    ) -> bool:
        """Closes the client, propagating any exception raised in the block."""
        ...

    def __repr__(self) -> str:
        """Returns a detailed string representation of the client."""
        ...

    def __str__(self) -> str:
        """Returns a human-readable string representation."""
        ...
