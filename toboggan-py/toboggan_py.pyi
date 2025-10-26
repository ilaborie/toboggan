"""Toboggan for Python.

This module provides Python bindings for the Toboggan presentation system,
enabling real-time multi-client synchronization via WebSocket connections.
"""

from typing import final

__all__ = ["Talk", "Slides", "State", "Toboggan"]

@final
class Talk:
    """Presentation metadata.

    Contains information about the presentation including title, date,
    optional footer content, and a list of all slide titles.

    Note:
        This class cannot be instantiated directly. Obtain instances
        via the `Toboggan.talk` property.
    """

    def __repr__(self) -> str:
        """Returns a detailed string representation of the talk metadata."""
        ...

    def __str__(self) -> str:
        """Returns the presentation title."""
        ...

@final
class Slides:
    """Collection of slides in the presentation.

    Contains all slides with their content, metadata, and ordering.
    Slides can include text, HTML, markdown, iframes, and layout containers.

    Note:
        This class cannot be instantiated directly. Obtain instances
        via the `Toboggan.slides` property.
    """

    def __str__(self) -> str:
        """Returns a list of slide titles."""
        ...

@final
class State:
    """Current presentation state.

    Represents the real-time state of the presentation, synchronized across
    all connected clients. The state can be:
    - Init: Initial state before presentation starts
    - Paused: Presentation is paused with current slide and duration
    - Running: Presentation is actively running with current slide and timing

    Note:
        This class cannot be instantiated directly. Obtain instances
        via the `Toboggan.state` property.
    """

    def __repr__(self) -> str:
        """Returns a detailed string representation of the current state."""
        ...

@final
class Toboggan:
    """Toboggan presentation client.

    Main client for connecting to a Toboggan presentation server.
    Manages WebSocket communication, state synchronization, and provides
    methods for controlling the presentation (navigation, playback).

    The client automatically maintains a persistent connection to the server
    and synchronizes state changes across all connected clients in real-time.

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

        # Check current state
        print(client.state)
        ```
    """

    def __init__(self, host: str = "localhost", port: int = 8080) -> None:
        """Creates a new Toboggan client and connects to the server.

        Args:
            host: Server hostname or IP address (default: "localhost")
            port: Server port number (default: 8080)

        Raises:
            ConnectionError: If connection to server fails or metadata cannot be fetched.
        """
        ...

    @property
    def talk(self) -> Talk:
        """Presentation metadata.

        Returns information about the presentation including title, date,
        footer content, and all slide titles.
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
        paused, or in initial state.

        This property reflects the state synchronized across all connected
        clients. Changes made by any client will be reflected here.
        """
        ...

    def previous(self) -> None:
        """Navigates to the previous slide.

        Sends a command to move backward in the presentation.
        This change will be synchronized across all connected clients.
        """
        ...

    def next(self) -> None:
        """Navigates to the next slide.

        Sends a command to move forward in the presentation.
        This change will be synchronized across all connected clients.
        """
        ...

    def __repr__(self) -> str:
        """Returns a detailed string representation of the client."""
        ...

    def __str__(self) -> str:
        """Returns a human-readable string representation."""
        ...
