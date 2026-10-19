## Directive

A Pydantic model under `api/**/schemas/requests/` is named `{Name}Request`. A warning by default.

## Why

The suffix says at the call site which side of the boundary a model is on, and keeps a request
model from being returned by mistake.

## Do

```python
class CreateContractRequest(BaseModel):
    name: str
```

## Don't

```python
class CreateContract(BaseModel):
    name: str
```

## Suppression

A comment on the `class` line silences the finding.

```python
class CreateContract(BaseModel):  # botica: ignore[api-request-schema-name]
class CreateContract(BaseModel):  # noqa: BTFAST003 -- reason
```
