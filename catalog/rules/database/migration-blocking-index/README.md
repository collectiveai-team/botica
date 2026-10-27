## Directive

The rule reads the Alembic migrations, the files `**/versions/*.py`, and reports these calls in the
body of `upgrade()`:

- `op.create_index` on a table that already exists, without `postgresql_concurrently=True`;
- `op.create_index` or `op.drop_index` with `postgresql_concurrently=True` outside a
  `with op.get_context().autocommit_block():` block.

One message covers both: the fix is the same. A warning by default.

## Why

On PostgreSQL, `CREATE INDEX` "locks the table to be indexed against writes" until the index is
built, so the running version cannot insert, update or delete rows of the table meanwhile.
`CREATE INDEX CONCURRENTLY` lets writes go on, and "cannot run inside a transaction block". The
`env.py` that `alembic init` writes runs the migrations in a transaction on PostgreSQL, and
`op.get_context().autocommit_block()` runs its body outside of it.

## Exceptions

The rule does not report:

- a call in `downgrade()`;
- `op.create_index` on a table that `op.create_table`, or a function named `create_table`, creates
  in the same `upgrade()`: the table is empty and no running code writes to it;
- a file outside a `versions/` folder;
- `batch_op.create_index` of `op.batch_alter_table`, SQL in `op.execute`, and calls through a
  helper function of the project.

On a database other than PostgreSQL, `postgresql_concurrently=True` changes nothing, and
`autocommit_block()` still runs.

## Do

```python
def upgrade() -> None:
    with op.get_context().autocommit_block():
        op.create_index("ix_users_email", "users", ["email"], postgresql_concurrently=True)
```

## Don't

```python
def upgrade() -> None:
    op.create_index("ix_users_email", "users", ["email"])
```

## Suppression

One comment silences one finding, on the first line of the call. There are two forms:
`# botica: ignore[<slug>]` names the rule by its slug, and `# noqa: <code> -- <reason>` names it by
its code, with the reason after `--`.

```python
op.create_index(  # botica: ignore[migration-blocking-index] -- MySQL branch
    "ix_log_level", "log", ["level"]
)
op.create_index("ix_log_level", "log", ["level"])  # noqa: BTDB005 -- MySQL branch
```
