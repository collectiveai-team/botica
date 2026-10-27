---
id: CES-0
title: A migration keeps the running version working, and its downgrade undoes it
references: ["database"]
---

## Directive

An Alembic migration changes the schema in a way that the version of the program that still runs
can use, and its `downgrade()` undoes its `upgrade()`.

- `upgrade()` adds and does not take away. A column or a table is renamed in three steps: a
  migration adds the new one, a release of the program writes and reads it, and a later migration
  drops the old one. A column or a table is dropped in a migration of its own, once no running code
  reads it.
- An index on a table that already exists is built with `postgresql_concurrently=True`, inside
  `with op.get_context().autocommit_block():`. An index on a table that the same migration creates
  needs neither.
- `downgrade()` holds the operations that undo `upgrade()`. When the migration cannot be undone,
  because its upgrade deletes data, `downgrade()` raises `NotImplementedError` with the reason.

## Why

CES-18 runs the migrations as a step of the deployment, before the new version of the program
replaces the one that runs. Until the replacement ends, the old version runs on the new schema. A
column that the migration renamed or dropped makes the queries of the old version fail, and a
dropped column takes its data with it.

On PostgreSQL 18, `ALTER TABLE` takes an `ACCESS EXCLUSIVE` lock unless its documentation says
otherwise, and `CREATE INDEX` "locks the table to be indexed against writes" while it builds the
index. `CREATE INDEX CONCURRENTLY` lets writes go on, but "a regular CREATE INDEX command can be
performed within a transaction block, but CREATE INDEX CONCURRENTLY cannot". The `env.py` that
`alembic init` writes runs the migrations in a transaction on PostgreSQL, and
`MigrationContext.autocommit_block()` runs its body outside of it,
"for the occasional database DDL or system operation that specifically has to be run outside of
any kind of transaction block". The SQLAlchemy documentation of the PostgreSQL dialect says the
same of `postgresql_concurrently`: "the PostgreSQL database requires that the statement be invoked
outside of a transaction block".

`alembic downgrade` runs `downgrade()` and then moves the revision of the database back. A
`downgrade()` that does nothing leaves the change in the database while the database records it as
undone, and the next `alembic upgrade` fails on it. `raise NotImplementedError` stops the downgrade
at that migration with its reason.

squawk lints the SQL of a migration for the same cases of `upgrade()` (`ban-drop-column`,
`ban-drop-table`, `renaming-column`, `renaming-table`, `require-concurrent-index-creation`,
`ban-concurrent-index-creation-in-transaction`). Its recipe for Alembic pipes the output of
`alembic upgrade head --sql` into it, which runs `env.py` and needs the dialect of the database.
The rules of this standard read the Python of the migration file, with no `env.py`, and also read
`downgrade()`.

Sources:

- PostgreSQL 18, CREATE INDEX: <https://www.postgresql.org/docs/current/sql-createindex.html>
- PostgreSQL 18, ALTER TABLE: <https://www.postgresql.org/docs/current/sql-altertable.html>
- Alembic, `MigrationContext.autocommit_block()`:
  <https://alembic.sqlalchemy.org/en/latest/api/runtime.html#alembic.runtime.migration.MigrationContext.autocommit_block>
- SQLAlchemy, Indexes with CONCURRENTLY:
  <https://docs.sqlalchemy.org/en/20/dialects/postgresql.html#indexes-with-concurrently>
- squawk, `require-concurrent-index-creation`:
  <https://squawkhq.com/docs/require-concurrent-index-creation>

## Examples

```python
def upgrade() -> None:
    op.add_column("users", sa.Column("full_name", sa.String(), nullable=True))
    with op.get_context().autocommit_block():
        op.create_index("ix_users_email", "users", ["email"], postgresql_concurrently=True)


def downgrade() -> None:
    with op.get_context().autocommit_block():
        op.drop_index("ix_users_email", table_name="users", postgresql_concurrently=True)
    op.drop_column("users", "full_name")
```

```python
def upgrade() -> None:
    op.alter_column("users", "name", new_column_name="full_name")
    op.create_index("ix_users_email", "users", ["email"])


def downgrade() -> None:
    pass
```

The first migration adds a column that the running version does not read, and builds its index
while the table takes writes. The second one breaks every query of the running version that reads
`users.name`, blocks the writes to `users` until the index exists, and cannot be undone.
