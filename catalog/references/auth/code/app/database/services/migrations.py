"""The migrations of `app/database/migrations/`, applied from Python."""

from __future__ import annotations

from typing import TYPE_CHECKING

from alembic import command
from alembic.config import Config

if TYPE_CHECKING:
    from sqlalchemy import Connection


def migration_config(connection: Connection | None = None) -> Config:
    """Return the configuration of Alembic, with the migrations read from the installed package.

    With a connection, `env.py` runs the migrations on it instead of opening its own.
    """
    config = Config()
    config.set_main_option("script_location", "app.database:migrations")
    if connection is not None:
        config.attributes["connection"] = connection
    return config


def upgrade_to_head(connection: Connection | None = None) -> None:
    """Apply the migrations that the database does not have yet."""
    command.upgrade(migration_config(connection), "head")
