"""The engine of the process and the session of each request."""

from __future__ import annotations

from functools import lru_cache
from typing import TYPE_CHECKING

from sqlmodel import Session, create_engine

from app.database.settings import get_settings

if TYPE_CHECKING:
    from collections.abc import Iterator

    from sqlalchemy import Engine


@lru_cache
def get_engine() -> Engine:
    """Return the engine of the process, built once from the settings of the database."""
    settings = get_settings()
    return create_engine(settings.url, echo=settings.echo)


def get_session() -> Iterator[Session]:
    """Yield a session for one request and close it after the response."""
    with Session(get_engine()) as session:
        yield session
