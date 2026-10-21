## Directive

Every Pydantic request or response model at the API boundary sets
`model_config = ConfigDict(extra="forbid")`. A warning by default. The rule looks at any
`BaseModel` subclass under `**/api/**/schemas/requests/**` or `**/api/**/schemas/responses/**`
(versioned `v*` directories included) that leaves it out.

## Why

- Without `extra="forbid"` Pydantic silently drops unknown fields: a client that typos `emial`
  or sends a stale field gets a 200, and the value is ignored.
- With `extra="forbid"` the schema accepts only its declared fields, and an unknown field gets a
  422.

## Exceptions

The rule goes by the path of a file: it reads the schemas under an api `schemas/requests` or
`schemas/responses` package. Domain models, and every model of a repository without an API, are
left alone.

## Do

```python
from pydantic import BaseModel, ConfigDict


class CreateUserRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")

    email: str
```

`extra="forbid"` may sit next to other settings: `ConfigDict(frozen=True, extra="forbid")`.

## Don't

```python
class CreateUserRequest(BaseModel):
    email: str
```

## Suppression

A comment on the `class` line silences the finding.

```python
class CreateUserRequest(BaseModel):  # botica: ignore[api-schemas-extra-forbid]
class CreateUserRequest(BaseModel):  # noqa: BTFAST001 -- reason
```
