---
id: CES-4
title: API schemas forbid extra fields
references: ["api-schemas"]
---

## Directive

Every request and response model of a public API sets `model_config = ConfigDict(extra="forbid")`.
Which models are public is answered by where they live: the schema modules of the API boundary
(`api/schemas/requests/` and `api/schemas/responses/`, see CES-17). Internal and domain models are
out of scope.

```python
class CreateUserRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")

    email: str
```

## Why

`extra="forbid"` rejects unexpected fields, so a client typo or a drift of the contract returns a
validation error. The standard is scoped by placement: a model is public when it lives in the
schema modules of the API boundary.
