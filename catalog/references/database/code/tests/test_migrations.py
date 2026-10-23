"""The migrations build the schema that the tables declare, and undo it."""

from __future__ import annotations

from typing import TYPE_CHECKING

from alembic import command
from alembic.autogenerate import compare_metadata
from alembic.migration import MigrationContext
from sqlalchemy import inspect
from sqlmodel import SQLModel

import app.database.models  # noqa: F401
from app.database.services.migrations import migration_config

if TYPE_CHECKING:
    from sqlalchemy import Engine


def test_the_migrations_leave_nothing_for_autogenerate_to_add(engine: Engine):
    with engine.connect() as connection:
        diff = compare_metadata(MigrationContext.configure(connection), SQLModel.metadata)

    assert diff == []


def test_the_database_is_at_the_last_revision(engine: Engine):
    with engine.connect() as connection:
        assert MigrationContext.configure(connection).get_current_revision() == "0001"


def test_downgrade_to_base_removes_the_tables(engine: Engine):
    with engine.begin() as connection:
        command.downgrade(migration_config(connection), "base")

    assert inspect(engine).get_table_names() == ["alembic_version"]
