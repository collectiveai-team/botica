"""Drop the table audit_log.

Revision ID: 0005
Revises: 0004
Create Date: 2026-10-27
"""

from __future__ import annotations

from alembic import op

revision = "0005"
down_revision = "0004"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.drop_table("audit_log")  # botica: ignore[migration-drop-or-rename] -- unread since 0003


def downgrade() -> None:
    raise NotImplementedError("one-way: the rows of audit_log are gone; restore them from a backup")
