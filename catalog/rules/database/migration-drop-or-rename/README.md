## Directive

The rule reads the Alembic migrations, the files `**/versions/*.py`, and reports these calls in the
body of `upgrade()`:

- `op.drop_column` and `op.drop_table`;
- `op.rename_table`, and `op.alter_column` with `new_column_name=`.

A warning by default.

## Why

The deployment runs `alembic upgrade head` before the new version of the program replaces the one
that runs. Until then the old version runs on the new schema: a column or a table that the
migration dropped or renamed makes its queries fail, and a dropped column or table loses its data.

A column or a table is renamed in three steps: a migration adds the new one, a release of the
program writes and reads it, and a later migration drops the old one.

## Exceptions

The rule does not report:

- a call in `downgrade()`, which undoes a change;
- `op.drop_table` on a table that `op.create_table`, or a function named `create_table`, creates in
  the same `upgrade()`, such as a staging table;
- a file outside a `versions/` folder;
- the operations of `op.batch_alter_table`, which rebuild the table on SQLite;
- SQL in `op.execute`, and calls through a helper function of the project.

## Do

```python
def upgrade() -> None:
    op.add_column("users", sa.Column("full_name", sa.String(), nullable=True))
```

## Don't

```python
def upgrade() -> None:
    op.alter_column("users", "name", new_column_name="full_name")
```

## Suppression

The migration that drops the old column or table, once no running code reads it, or a migration of
a program that is stopped while it migrates, carries a suppression with the reason. One comment
silences one finding, on the first line of the call. There are two forms:
`# botica: ignore[<slug>]` names the rule by its slug, and `# noqa: <code> -- <reason>` names it by
its code, with the reason after `--`.

```python
op.drop_column("users", "name")  # botica: ignore[migration-drop-or-rename] -- unread since 0012
op.drop_column("users", "name")  # noqa: BTDB003 -- unread since 0012
```
