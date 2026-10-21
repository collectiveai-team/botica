"""Does no analysis: measures the protocol and process overhead on its own."""

import sys
from collections.abc import Iterator
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "sdk" / "python"))

from botica.sdk import rule, serve


@rule("bench/noop", ["python"])
def noop(path: str, text: str) -> Iterator[tuple[int, int, str]]:
    return iter(())


if __name__ == "__main__":
    serve()
