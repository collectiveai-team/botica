# CES-4 · API schemas forbid extra fields

**Code:** `CES-4` &nbsp;·&nbsp; **Slug:** `api-schemas-extra-forbid` &nbsp;·&nbsp; **Enforced by:**
ast-grep (`warning`, placement-scoped) &nbsp;·&nbsp; **Tracker:**
[#4](https://github.com/collectiveai-team/botica/issues/4)

## Directive

Every Pydantic request/response model at the API boundary must set
`model_config = ConfigDict(extra="forbid")`. The rule fires on any `BaseModel` subclass under
`**/api/**/schemas/requests/**` or `**/api/**/schemas/responses/**` (including versioned `v*`
directories) that omits it.

## Why

- Without `extra="forbid"`, Pydantic **silently drops** unknown fields. A client that typos
  `emial` or sends a stale field gets a 200 with the value quietly ignored — a data-loss bug
  that no test catches.
- Forbidding extras turns the schema into a strict, self-documenting contract: the accepted
  shape is exactly the declared fields, and a mismatch fails loudly with a 422.

## Placement, not intent

Differentiation is by **placement** (per the CES-17 api-boundary layout), never by guessing
intent. Only schemas physically under the api `schemas/{requests,responses}` packages are
checked. Internal/domain `BaseModel`s — and every model in a non-API repo — are inert, so the
rule never nags where it doesn't belong.

## Do

```python
from pydantic import BaseModel, ConfigDict

class CreateUserRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    email: str
```

`extra="forbid"` may sit alongside other config (`ConfigDict(frozen=True, extra="forbid")`).
See `.agents/snippets/api-schemas.py` for the canonical request + response pair.

## Don't

```python
class CreateUserRequest(BaseModel):   # api-schemas-extra-forbid
    email: str
```

## Suppressing (rare, must be visible)

A boundary model that genuinely must accept arbitrary keys (e.g. a passthrough webhook) keeps
the `BaseModel` and adds a visible ignore on the class line:

```python
class WebhookEnvelope(BaseModel):  # ast-grep-ignore: api-schemas-extra-forbid
    ...
```

The slug is the stable suppression key; the `CES-4` code in the message is for humans.
