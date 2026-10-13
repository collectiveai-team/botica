# api-schema-in-route-module · `BTFAST001`

**Standard:** none yet · **Family:** fastapi · **Languages:** Python · **Status:** proposed, not in the catalog

## Directive

A route module holds routes. A Pydantic schema (`BaseModel`) defined in a module under
`api/**/routers/` or `api/**/routes/` is reported: put it in the `schemas/requests/` or
`schemas/responses/` package of `api`. A warning by default.

## Why

A schema inside a route file cannot be reused by another route or by a client, and the
request/response split that the `schemas` packages express is lost.

## Do

```python
# api/routers/contracts.py
from myapp.api.schemas.requests.contract import CreateContractRequest
```

## Don't

```python
# api/routers/contracts.py
class CreateContract(BaseModel):
    name: str
```

## Suppression

Silence one line with `# botica: ignore[api-schema-in-route-module]` or `# noqa: BTFAST001`.
