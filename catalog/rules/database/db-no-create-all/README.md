## Directive

`metadata.create_all(...)` is reported outside the tests: the schema evolves through Alembic
migrations only. A warning by default.

## Why

`create_all` builds the tables the models describe today and skips the ones a migration would
have changed, so the schema of a database and its migration history drift apart without a trace.

## Do

```python
command.upgrade(config, "head")
```

## Don't

```python
SQLModel.metadata.create_all(engine)
```

## Suppression

A comment on the line of the `create_all(...)` call silences it.

```python
SQLModel.metadata.create_all(engine)  # botica: ignore[db-no-create-all]
SQLModel.metadata.create_all(engine)  # noqa: BTDB002 -- reason
```
