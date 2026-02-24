"""GUI state machine."""

from enum import Enum, auto


class State(Enum):
    LOADING = auto()
    LISTENING = auto()
    TRANSCRIBING = auto()
    RESULT = auto()
    ERROR = auto()
