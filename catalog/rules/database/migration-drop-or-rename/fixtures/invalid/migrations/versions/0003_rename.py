"""Rename the column name of users and the table orders.

Revision ID: 0003
Revises: 0002
Create Date: 2026-10-27
"""

from __future__ import annotations

from alembic import op

revision = "0003"
down_revision = "0002"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.alter_column("users", "name", new_column_name="full_name")
    op.rename_table("orders", "purchases")


def downgrade() -> None:
    op.rename_table("purchases", "orders")
    op.alter_column("users", "full_name", new_column_name="name")
