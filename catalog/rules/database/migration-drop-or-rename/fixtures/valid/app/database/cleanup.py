"""Drop the tables of a test database. It lives outside versions/, so it is not a migration."""

from __future__ import annotations

from alembic import op


def upgrade() -> None:
    op.drop_table("audit_log")
    op.drop_column("users", "legacy_id")
    op.create_index("ix_users_email", "users", ["email"])
