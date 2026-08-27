---
title: Use in-memory or local adapters in tests — no live services
section: test
scope: general
applies-to: all
status: direction
tags: test, fixtures, adapters, sqlite, fastapi
---

## Use in-memory or local adapters in tests — no live services

Tests must not require a running database, cloud credentials, or network access. Override
external dependencies at the boundary using an in-memory equivalent. The canonical pattern
for FastAPI + SQLModel: create a SQLite in-memory engine per test session, bootstrap all
tables with `SQLModel.create_all`, and override the real `get_db_session` dependency on the
`TestClient` so every route hits the same in-memory session.

**Prefer:**

```python
# tests/conftest.py
import pytest
from sqlmodel import Session, SQLModel, create_engine
from sqlmodel.pool import StaticPool
from fastapi.testclient import TestClient

from myapp.api.main import app
from myapp.core.database.session import get_db_session

@pytest.fixture(name="session")
def session_fixture():
    engine = create_engine(
        "sqlite://",
        connect_args={"check_same_thread": False},
        poolclass=StaticPool,
    )
    SQLModel.metadata.create_all(engine)
    with Session(engine) as session:
        yield session

@pytest.fixture(name="client")
def client_fixture(session: Session):
    app.dependency_overrides[get_db_session] = lambda: session
    yield TestClient(app)
    app.dependency_overrides.clear()
```

Split tests into `tests/api/` (endpoint tests using the `client` fixture) and
`tests/core/` (unit tests using the `session` fixture directly).

**Avoid:**

```python
# Hitting a real Postgres URL — breaks in CI without the service running
engine = create_engine(os.getenv("DATABASE_URL"))
```

Reference: see `test-coverage-gap` for the overall testing baseline; `arch-interface-is-test-surface`
for what to assert through the `TestClient`.
