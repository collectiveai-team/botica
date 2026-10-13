# api-response-schema-name · `BTFAST003`

**Standard:** none yet · **Family:** fastapi · **Languages:** Python · **Status:** proposed, not in the catalog

## Directive

A Pydantic model under `api/**/schemas/responses/` is named `{Name}Response`. A warning by default.

## Why

The suffix says at the call site which side of the boundary a model is on, and keeps a table
class from being returned by mistake.

## Do

```python
class ContractResponse(BaseModel):
    id: str
```

## Don't

```python
class Contract(BaseModel):
    id: str
```

## Suppression

Silence one line with `# botica: ignore[api-response-schema-name]` or `# noqa: BTFAST003`.
