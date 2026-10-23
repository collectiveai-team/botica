"""A blocking call that the code of `app` makes while an event loop runs fails the test."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from blockbuster import BlockBuster, blockbuster_ctx

if TYPE_CHECKING:
    from collections.abc import Iterator


@pytest.fixture(autouse=True)
def blockbuster() -> Iterator[BlockBuster]:
    with blockbuster_ctx("app") as bb:
        yield bb
