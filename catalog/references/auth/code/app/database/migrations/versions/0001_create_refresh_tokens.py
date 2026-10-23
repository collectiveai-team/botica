"""Create the table of refresh tokens.

Revision ID: 0001
Revises:
Create Date: 2026-10-22
"""

from __future__ import annotations

import sqlalchemy as sa
import sqlmodel
from alembic import op

revision = "0001"
down_revision = None
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_table(
        "refresh_tokens",
        sa.Column("token_hash", sqlmodel.AutoString(length=64), nullable=False),
        sa.Column("user_id", sqlmodel.AutoString(length=255), nullable=False),
        sa.Column("family", sqlmodel.AutoString(length=64), nullable=False),
        sa.Column("expires_at", sa.DateTime(timezone=True), nullable=False),
        sa.Column("used", sa.Boolean(), nullable=False),
        sa.PrimaryKeyConstraint("token_hash"),
    )
    op.create_index(op.f("ix_refresh_tokens_family"), "refresh_tokens", ["family"], unique=False)


def downgrade() -> None:
    op.drop_index(op.f("ix_refresh_tokens_family"), table_name="refresh_tokens")
    op.drop_table("refresh_tokens")
