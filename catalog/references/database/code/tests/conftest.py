"""A database in memory, built by the migrations of the package and not by `create_all`."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from sqlalchemy.pool import StaticPool
from sqlmodel import Session, create_engine

from app.database.services.migrations import upgrade_to_head

if TYPE_CHECKING:
    from collections.abc import Iterator

    from sqlalchemy import Engine


@pytest.fixture
def engine() -> Iterator[Engine]:
    # One connection for the whole test, shared with the threads of TestClient: each new connection
    # to `sqlite://` is a new, empty database.
    engine = create_engine(
        "sqlite://", poolclass=StaticPool, connect_args={"check_same_thread": False}
    )
    with engine.begin() as connection:
        upgrade_to_head(connection)
    yield engine
    engine.dispose()


@pytest.fixture
def session(engine: Engine) -> Iterator[Session]:
    with Session(engine) as session:
        yield session
