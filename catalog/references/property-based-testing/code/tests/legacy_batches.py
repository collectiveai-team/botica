"""The implementation of `batches` before its refactor, kept as the oracle of the new one."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence


def legacy_batches[T](items: Sequence[T], size: int) -> list[list[T]]:
    if size < 1:
        msg = f"the size of a batch is 1 or more, not {size}"
        raise ValueError(msg)
    result: list[list[T]] = []
    current: list[T] = []
    for item in items:
        current.append(item)
        if len(current) == size:
            result.append(current)
            current = []
    if current:
        result.append(current)
    return result
