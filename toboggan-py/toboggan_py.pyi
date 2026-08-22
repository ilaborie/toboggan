"""Toboggan for Python.

This module provides Python bindings for the Toboggan presentation system,
enabling real-time multi-client synchronization via WebSocket connections.
"""

from typing import List, Optional, final

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

        Either empty — meaning the server did not compute them — or exactly as
        long as `titles`, read against it by index.
        """
        ...

    @property
    def durations(self) -> List[Optional[int]]:
        """Planned speaking time per slide, in seconds.

        From each slide's `duration` front matter, and None where the author
        did not declare one. Either empty or exactly as long as `titles`.
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
    def kind(self) -> str:
        """`"Cover"`, `"Part"` for a section title, or `"Standard"`."""
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
        """Render targets this slide is excluded from: `"web"`, `"pdf"`.

        Empty means the slide is visible everywhere.
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
        """Returns the slide at `index`, or None if out of range."""
        ...

    def __len__(self) -> int:
        """Returns the number of slides."""
        ...

    def __getitem__(self, index: int) -> Slide:
        """Returns the slide at `index`.

        Raises:
            IndexError: If `index` is out of range.
        """
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

    def is_first_slide(self, total_slides: int) -> bool:
        """Whether the deck is on its first slide.

        Args:
            total_slides: How many slides the deck has — `len(client.slides)`.
                An empty deck is on neither its first nor its last slide.
        """
        ...

    def is_last_slide(self, total_slides: int) -> bool:
        """Whether the deck is on its last slide.

        Args:
            total_slides: How many slides the deck has — `len(client.slides)`.
        """
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
        """Where the connection came from, which is also what decided its role."""
        ...

    @property
    def role(self) -> str:
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
    methods for controlling the presentation (navigation, playback).

    The client automatically maintains a persistent connection to the server
    and synchronizes state changes across all connected clients in real-time.

    Roles:
        A client never claims a role — it offers a presenter token and the
        server decides. A connection from the machine running the server
        presents; a connection from anywhere else presents only if it carries
        the token. Check `is_presenter` before relying on navigation: an
        audience client's commands are refused by the server.

    Example:
        ```python
        from toboggan_py import Toboggan

        # Connect to server
        client = Toboggan("localhost", 8080)

        # Access presentation metadata
        print(client.talk)
        print(client.slides)

        # Navigate slides
        client.next()
        client.previous()
        client.goto(12)

        # Check current state
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
            ConnectionError: If connection to server fails or metadata cannot
                be fetched. A registration the server has not yet answered is
                not an error — `role` reports None until it arrives.
        """
        ...

    @property
    def talk(self) -> Talk:
        """Presentation metadata.

        Returns information about the presentation including title, date,
        language, footer content, and all slide titles.
        """
        ...

    @property
    def slides(self) -> Slides:
        """All slides in the presentation.

        Returns the complete collection of slides with their content,
        metadata, and ordering.
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
        """
        ...

    @property
    def role(self) -> Optional[str]:
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

        Sends a command to move backward in the presentation.
        This change will be synchronized across all connected clients.
        """
        ...

    def next(self) -> None:
        """Navigates to the next slide, skipping any reveals left on this one.

        Sends a command to move forward in the presentation.
        This change will be synchronized across all connected clients.
        """
        ...

    def first(self) -> None:
        """Navigates to the first slide."""
        ...

    def last(self) -> None:
        """Navigates to the last slide."""
        ...

    def goto(self, slide: int) -> None:
        """Navigates to a specific slide.

        Args:
            slide: The slide number as printed on the slide, counting from 1.
        """
        ...

    def next_step(self) -> None:
        """Reveals the next step, moving to the next slide once this one runs out.

        This is what a presenter remote and the space bar send.
        """
        ...

    def previous_step(self) -> None:
        """Goes back one step, moving to the previous slide once this one runs out."""
        ...

    def blink(self) -> None:
        """Flashes every other client, to get the room's attention."""
        ...

    def clients(self) -> List[ClientInfo]:
        """Lists who is currently connected.

        Presenter-only on the server — it reports names, roles and IP
        addresses — so an audience client cannot enumerate the room.

        Raises:
            ConnectionError: If the server cannot be reached, or if it refuses
                the request because this client is not a presenter.
        """
        ...

    def __repr__(self) -> str:
        """Returns a detailed string representation of the client."""
        ...

    def __str__(self) -> str:
        """Returns a human-readable string representation."""
        ...
