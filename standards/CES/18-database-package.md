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
└── settings.py     database configuration (CES-76)
```

`core` holds the pure domain models and the repository protocol; `database` holds the tables and
the concrete repositories. `sqlmodel`, `sqlalchemy` and `alembic` are imported only there.

The schema changes only through a migration of `database/migrations/`. Code that runs in production
does not create tables with `metadata.create_all()`; a test may.

The migrations run as a step of the deployment, once, before the new version of the API starts or
replaces the running one: a job, an init container or a release command that runs
`alembic upgrade head` or a module of `entrypoints` that calls it. A process of the API does not
apply migrations when it starts, in its lifespan or in any other hook.

## Why

The persistence libraries are imported in one package, so the domain imports no database library
and its tests run without a database. The boundary between the domain and its storage is a
directory.

Alembic takes no lock. Replicas that each migrate when they start run the same DDL at the same time
and conflict, and a migration that fails leaves processes that do not start. A step of its own runs
once, and when it fails the deployment stops and the running version keeps serving.
