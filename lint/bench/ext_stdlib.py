"""Same rule as ext_bare_except.py using only the stdlib `ast`; isolates uv/ast-grep-py cost."""

import ast
import sys
from collections.abc import Iterator
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "python"))

from botica.sdk import rule, serve


@rule("bench/no-bare-except", ["python"])
def no_bare_except(path: str, text: str) -> Iterator[tuple[int, int, str]]:
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return
    lines = text.splitlines(keepends=True)
    offsets = [0]
    for line in lines:
        offsets.append(offsets[-1] + len(line.encode()))
    for node in ast.walk(tree):
        if isinstance(node, ast.ExceptHandler) and node.type is None:
            start = offsets[node.lineno - 1] + node.col_offset
            yield (
                start,
                start + 7,
                "bench/no-bare-except: catch a specific exception, not a bare `except:`.",
            )


if __name__ == "__main__":
    serve()
