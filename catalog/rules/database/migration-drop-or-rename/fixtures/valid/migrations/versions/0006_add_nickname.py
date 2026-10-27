"""Add the column nickname to users.

Revision ID: 0006
Revises: 0005
Create Date: 2026-10-27
"""

from __future__ import annotations

import sqlalchemy as sa
from alembic import op

revision = "0006"
down_revision = "0005"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.add_column("users", sa.Column("nickname", sa.String(), nullable=True))
    op.alter_column("users", "email", existing_type=sa.String(length=254), nullable=True)


def downgrade() -> None:
    op.alter_column("users", "email", existing_type=sa.String(length=254), nullable=False)
    op.drop_column("users", "nickname")
