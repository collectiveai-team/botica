"""Fill the nicknames of users with their emails.

Revision ID: 0007
Revises: 0006
Create Date: 2026-10-27
"""

from __future__ import annotations

from alembic import op

revision = "0007"
down_revision = "0006"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.execute("UPDATE users SET nickname = email WHERE nickname IS NULL")


def downgrade() -> None:
    pass
