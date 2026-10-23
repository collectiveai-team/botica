"""The environment of Alembic: the metadata to compare and the connection to migrate."""

from __future__ import annotations

from typing import TYPE_CHECKING

from alembic import context
from sqlmodel import SQLModel, create_engine

import app.database.models  # noqa: F401  # registers every table on SQLModel.metadata
from app.database.settings import get_settings

if TYPE_CHECKING:
    from sqlalchemy import Connection

target_metadata = SQLModel.metadata


def run_migrations_offline() -> None:
    """Write the SQL of the migrations instead of running it (`alembic upgrade --sql`)."""
    context.configure(url=get_settings().url, target_metadata=target_metadata, literal_binds=True)
    with context.begin_transaction():
        context.run_migrations()


def run_on(connection: Connection) -> None:
    """Run the migrations on one connection, in one transaction."""
    context.configure(connection=connection, target_metadata=target_metadata)
    with context.begin_transaction():
        context.run_migrations()


def run_migrations_online() -> None:
    """Run the migrations on the connection of the caller, or on one to the URL of the settings."""
    shared = context.config.attributes.get("connection")
    if shared is not None:
        run_on(shared)
        return
    with create_engine(get_settings().url).connect() as connection:
        run_on(connection)


if context.is_offline_mode():
    run_migrations_offline()
else:
    run_migrations_online()
