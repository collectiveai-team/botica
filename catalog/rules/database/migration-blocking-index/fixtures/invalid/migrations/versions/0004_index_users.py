"""Index the emails and the creation dates of users.

Revision ID: 0004
Revises: 0003
Create Date: 2026-10-27
"""

from __future__ import annotations

from alembic import op

revision = "0004"
down_revision = "0003"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.create_index("ix_users_email", "users", ["email"])
    op.create_index("ix_users_created_at", "users", ["created_at"], postgresql_concurrently=True)
    op.create_index("ix_users_name", table_name="users", columns=["name"])


def downgrade() -> None:
    op.drop_index("ix_users_name", table_name="users")
    op.drop_index("ix_users_created_at", table_name="users")
    op.drop_index("ix_users_email", table_name="users")
