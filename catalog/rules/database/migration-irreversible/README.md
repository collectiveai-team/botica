## Directive

The rule reads the Alembic migrations, the files `**/versions/*.py`, and reports a `downgrade()`
whose body holds only `pass`, `...`, a docstring or comments, when `upgrade()` changes the schema:
it calls an `op.create_*`, `op.drop_*`, `op.add_*`, `op.alter_*` or `op.rename_*` operation, or
`op.batch_alter_table`. A warning by default.

## Why

`alembic downgrade` runs `downgrade()` and then moves the revision of the database back. When
`downgrade()` does nothing, the database records the revision as undone and keeps its change, so
the next `alembic upgrade` runs the migration again on a schema that already has it, and fails.

A migration that cannot be undone, because its upgrade deletes data, says so: its `downgrade()`
raises `NotImplementedError` with the reason, and `alembic downgrade` stops there with that
message.

## Exceptions

The rule does not report:

- a migration whose `upgrade()` changes no schema, such as a merge of two heads or a migration of
  data through `op.execute`;
- a `downgrade()` that raises, or that holds any other statement;
- a file outside a `versions/` folder.

## Do

```python
def upgrade() -> None:
    op.drop_table("audit_log")


def downgrade() -> None:
    raise NotImplementedError("one-way: the rows of audit_log are gone; restore them from a backup")
```

## Don't

```python
def upgrade() -> None:
    op.add_column("users", sa.Column("nickname", sa.String(), nullable=True))


def downgrade() -> None:
    pass
```

## Suppression

One comment silences one finding, on the line of `def downgrade()`. There are two forms:
`# botica: ignore[<slug>]` names the rule by its slug, and `# noqa: <code> -- <reason>` names it by
its code, with the reason after `--`.

```python
def downgrade() -> None:  # botica: ignore[migration-irreversible] -- the upgrade repairs drift
def downgrade() -> None:  # noqa: BTDB004 -- the upgrade repairs drift
```
