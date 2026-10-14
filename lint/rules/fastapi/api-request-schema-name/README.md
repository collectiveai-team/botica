# api-request-schema-name · `BTFAST003`

**Standard:** none yet · **Family:** fastapi · **Languages:** Python · **Status:** preview, off by default (set `preview = true` and select it)

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

Silence one line with `# botica: ignore[api-request-schema-name]` or `# noqa: BTFAST003`.
