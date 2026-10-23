---
id: CES-18
title: Relational persistence is its own package
references: ["database"]
---

## Directive

Relational persistence is a unit of its own named `database`, outside `core`. It holds only SQL
concerns:

```text
database/
├── models/         SQLModel tables
├── repositories/   adapters that implement an interface of core
├── migrations/     alembic
├── services/       engine and session
└── settings/       database configuration
```

`core` holds the pure domain models and the repository protocol; `database` holds the tables and
the concrete repositories. `sqlmodel`, `sqlalchemy` and `alembic` are imported only there.

The schema changes only through a migration of `database/migrations/`. Code that runs in production
does not create tables with `metadata.create_all()`; a test may.

## Why

The persistence libraries are imported in one package, so the domain imports no database library
and its tests run without a database. The boundary between the domain and its storage is a
directory.
