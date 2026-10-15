---
id: CES-18
title: Relational persistence is its own package
---

## Directive

Relational persistence is a unit of its own named `database`, not `persistence`, not `meta`, and
not inside `core`. It holds only SQL concerns:

```text
database/
├── models/         SQLModel tables
├── repositories/   adapters that implement an interface of core
├── migrations/     alembic
├── services/       engine and session
└── settings/       database configuration
```

`core` keeps the pure domain models and the repository protocol; `database` keeps the tables and
the concrete repositories. `sqlmodel`, `sqlalchemy` and `alembic` are imported only there.

## Why

Confining the persistence libraries to one package keeps the domain testable without a database
and makes the boundary between the domain and its storage visible in the layout.

## Rules

- `arch-database-package`
