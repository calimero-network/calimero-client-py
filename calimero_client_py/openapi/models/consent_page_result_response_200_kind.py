from enum import Enum


class ConsentPageResultResponse200Kind(str, Enum):
    OUTBOUND = "outbound"
    RELEASE = "release"
    UPGRADE = "upgrade"

    def __str__(self) -> str:
        return str(self.value)
