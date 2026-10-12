# CES-18 · persistence lives in a database package

**Code:** `CES-18` &nbsp;·&nbsp; **Slug:** `arch-database-package` &nbsp;·&nbsp; **Enforced by:**
ast-grep (`warning`, placement-scoped) + a commented import-linter contract (CES-5, Slice 09)
&nbsp;·&nbsp; **Tracker:** [#18](https://github.com/collectiveai-team/botica/issues/18)

## Directive

Relational persistence lives in its own **`database`** unit — SQLModel tables, repositories,
migrations, and connection/session setup all together. It does **not** belong scattered under
`persistence/`, `meta/`, or `core/`. The ast-grep portion of this rule flags SQLModel table
definitions, `create_engine(...)`, and `sessionmaker(...)` found in those wrong homes.

## Why

- A single `database` package gives persistence one clear boundary: there is exactly one place
  that owns the schema, the engine, and the queries, so the rest of the app depends on an
  explicit seam rather than reaching into ORM internals from anywhere.
- `core/` and `meta/` are cross-cutting/utility homes; letting tables and engines leak into
  them erodes the layer direction (entrypoints → api → database/impl → core) that CES-5
  enforces.

## What the ast-grep portion catches

Scoped by `files:` to `**/persistence/**`, `**/meta/**`, `**/core/**`:

| Pattern | Why it's misplaced |
|---|---|
| `class X(SQLModel, table=True):` | a table outside the database package |
| `create_engine(...)` | connection setup outside the database package |
| `sessionmaker(...)` | session factory outside the database package |

It is inert in a correctly-placed `database/` package and in any repo without a relational
layer.

## Enforcement split

This slice ships the **deterministic ast-grep checks only**. The structural boundary direction
(who may import the database package) is encoded as a **commented `[tool.importlinter]`
skeleton** in `pyproject.toml`, shipped by Slice 09 (CES-5) — uncomment it once the package has
real layers. Active import-linter enforcement is intentionally out of scope here.

## Do

```text
myapp/
  database/
    __init__.py
    models.py        # SQLModel tables
    engine.py        # create_engine / sessionmaker
    repositories.py
    migrations/
```

## Don't

```python
# myapp/core/models.py
class User(SQLModel, table=True):   # arch-database-package
    ...
```

## Suppressing (rare, must be visible)

If a table genuinely must live elsewhere, keep it and add a visible ignore on the class/line:

```python
class AuditEntry(SQLModel, table=True):  # ast-grep-ignore: arch-database-package
    ...
```

The slug is the stable suppression key; the `CES-18` code in the message is for humans.
