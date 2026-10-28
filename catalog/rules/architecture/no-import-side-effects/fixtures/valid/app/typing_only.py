from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from pathlib import Path

    STUBS = open("stubs.txt")


class Loader:
    def load(self, path: "Path") -> bytes:
        return path.read_bytes()
