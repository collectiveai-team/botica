---
title: Assert behavior through the module's public interface, not its internals
section: test
scope: general
applies-to: all
status: direction
tags: test, architecture, interface, seam
---

## Assert behavior through the module's public interface, not its internals

Tests should cross the same seam that callers cross — the public function, HTTP endpoint,
or ABC method — and assert observable outputs. Tests that reach into private state or
patch internal helpers survive only as long as the current implementation; tests at the
interface survive refactors. This is the testing corollary of `arch-interface-is-test-surface`.

**Prefer (assert via the public seam):**

```python
# Test the HTTP interface — asserts behavior a real caller would observe
def test_create_contract_returns_id(client: TestClient):
    resp = client.post("/contracts/", json={"name": "NDA", "content": "..."})
    assert resp.status_code == 201
    assert "id" in resp.json()
```

**Avoid (reaching into internals):**

```python
# Breaks on any rename/move inside the module
from myapp.core.crud.contract import _build_insert_stmt
def test_internal_helper():
    stmt = _build_insert_stmt(name="NDA")
    assert "INSERT" in str(stmt)
```

The ABC or abstract interface layer is the natural seam for unit-testing adapters — test
against it, not against the concrete implementation's private helpers.

Reference: `arch-interface-is-test-surface`; `test-in-memory-adapters` for the fixture
setup that makes client-level tests fast.
