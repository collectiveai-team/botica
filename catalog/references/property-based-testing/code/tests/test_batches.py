"""The refactored `batches`, checked against its old implementation for any list and any size."""

from __future__ import annotations

from typing import TYPE_CHECKING

from hypothesis import given
from hypothesis import strategies as st
from tests.legacy_batches import legacy_batches

from app.core.batches import batches

if TYPE_CHECKING:
    from collections.abc import Callable


def outcome[**P, R](
    call: Callable[P, R], *args: P.args, **kwargs: P.kwargs
) -> tuple[str, R | type[Exception]]:
    try:
        return ("ok", call(*args, **kwargs))
    except Exception as exc:  # noqa: BLE001 -- the type of any exception is part of the outcome
        return ("raised", type(exc))


@given(st.lists(st.integers()), st.integers())
def test_the_new_batches_give_the_outcome_of_the_old_for_every_input(items: list[int], size: int):
    assert outcome(batches, items, size) == outcome(legacy_batches, items, size)
