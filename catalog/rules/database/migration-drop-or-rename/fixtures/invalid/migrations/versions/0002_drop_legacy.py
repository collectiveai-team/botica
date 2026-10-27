"""Drop the column legacy_id of users and the table audit_log.

Revision ID: 0002
Revises: 0001
Create Date: 2026-10-27
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import op

revision = "0002"
down_revision = "0001"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.drop_column("users", "legacy_id")
    op.drop_table("audit_log")


def downgrade() -> None:
    op.create_table("audit_log", sa.Column("id", sa.Integer(), primary_key=True))
    op.add_column("users", sa.Column("legacy_id", sa.Integer(), nullable=True))
