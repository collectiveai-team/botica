## Directive

Relational persistence lives in its own `database` package: SQLModel tables, repositories,
migrations and the connection and session setup, together. Not under `persistence/`, `meta/` or
`core/`. A warning by default.

The rule looks only at the files under `**/persistence/**`, `**/meta/**` and `**/core/**`, and
reports:

| Code | Why it is misplaced |
|---|---|
| `class X(SQLModel, table=True):` | a table outside the database package |
| `create_engine(...)` | connection setup outside the database package |
| `sessionmaker(...)` | a session factory outside the database package |

A correctly placed `database/` package, and a repository without a relational layer, are left
alone.

## Why

- One `database` package gives persistence a boundary: a single place owns the schema, the engine
  and the queries, and the rest of the application depends on an explicit seam.
- `core/` and `meta/` are cross-cutting homes. A table or an engine in them makes `core/`
  depend on the database layer.

## Do

```text
myapp/
  database/
    models/          # SQLModel tables
    repositories/    # adapters of the ports of core
    migrations/      # alembic
    services/        # create_engine, sessionmaker
    settings.py      # the settings of the package
```

## Don't

```python
# myapp/core/models.py
class User(SQLModel, table=True): ...
```

## Suppression

A comment on the line where the finding starts silences it: the `class` line, or the line of the
`create_engine(...)` or `sessionmaker(...)` call.

```python
class User(SQLModel, table=True):  # botica: ignore[database-package]
class User(SQLModel, table=True):  # noqa: BTDB001 -- reason
```
