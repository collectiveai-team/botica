"""Load the rows of a demo database. It lives outside versions/, so it is not a migration."""

from __future__ import annotations

from alembic import op


def upgrade() -> None:
    op.execute("INSERT INTO users (email) VALUES ('demo@example.com')")


def downgrade() -> None:
    pass
