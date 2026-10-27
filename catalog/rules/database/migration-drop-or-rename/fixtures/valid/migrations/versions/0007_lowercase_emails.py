"""Lowercase the emails of users through a staging table that the migration drops.

Revision ID: 0007
Revises: 0006
Create Date: 2026-10-27
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import op

revision = "0007"
down_revision = "0006"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_table(
        "_emails_staging",
        sa.Column("id", sa.Integer(), primary_key=True),
        sa.Column("email", sa.String(length=254), nullable=False),
    )
    op.execute("INSERT INTO _emails_staging SELECT id, lower(email) FROM users")
    op.execute(
        "UPDATE users SET email = s.email FROM _emails_staging AS s WHERE users.id = s.id"
    )
    op.drop_table("_emails_staging")


def downgrade() -> None:
    raise NotImplementedError("one-way: the original case of the emails is gone")
