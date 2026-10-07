# /// script
# requires-python = ">=3.10"
# dependencies = ["ast-grep-py"]
# ///
"""Spike extension: flags bare `except:` using ast-grep-py, over the botica protocol."""

import sys
from collections.abc import Iterator
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "sdk" / "python"))

from ast_grep_py import SgRoot
from botica_ext import char_to_byte_offsets, rule, serve


@rule("spike/no-bare-except", ["python"])
def no_bare_except(path: str, text: str) -> Iterator[tuple[int, int, str]]:
    to_byte = char_to_byte_offsets(text)
    for node in SgRoot(text, "python").root().find_all(kind="except_clause"):
        if node.text().startswith("except:"):
            span = node.range()
            yield (
                to_byte(span.start.index),
                to_byte(span.end.index),
                "spike/no-bare-except: catch a specific exception, not a bare `except:`.",
            )


if __name__ == "__main__":
    serve()
