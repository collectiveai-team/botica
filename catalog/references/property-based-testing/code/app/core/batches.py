"""The split of a list into batches of a fixed size."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence


def batches[T](items: Sequence[T], size: int) -> list[list[T]]:
    """Return the items in lists of `size` items, the last one shorter; `size` is 1 or more."""
    if size < 1:
        msg = f"the size of a batch is 1 or more, not {size}"
        raise ValueError(msg)
    return [list(items[start : start + size]) for start in range(0, len(items), size)]
