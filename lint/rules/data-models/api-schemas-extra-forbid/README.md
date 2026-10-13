# api-schemas-extra-forbid · `BTDM001`

**Standard:** CES-4 (documentation only) · **Family:** data-models · **Languages:** Python

## Directive

Every Pydantic request or response model at the API boundary sets
`model_config = ConfigDict(extra="forbid")`. A warning by default. The rule looks at any
`BaseModel` subclass under `**/api/**/schemas/requests/**` or `**/api/**/schemas/responses/**`
(versioned `v*` directories included) that leaves it out.

## Why

- Without `extra="forbid"` Pydantic silently drops unknown fields: a client that typos `emial`
  or sends a stale field gets a 200 and the value is ignored, a data loss no test catches.
- Forbidding extras makes the schema a strict contract: the accepted shape is exactly the
  declared fields, and a mismatch fails loudly with a 422.

## Where it applies

By placement, not by guessing intent: only schemas under an api `schemas/requests` or
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

Silence one line with `# botica: ignore[api-schemas-extra-forbid]` or `# noqa: BTDM001`.
