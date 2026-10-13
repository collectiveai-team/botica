# api-response-schema-name · `BTDM008`

**Standard:** none yet · **Family:** data-models · **Languages:** Python · **Status:** proposed, not in the catalog

## Directive

A Pydantic model under `api/**/schemas/responses/` is named `{Name}Out`. A warning by default.

## Why

The suffix says at the call site which side of the boundary a model is on, and keeps a table
class from being returned by mistake.

## Do

```python
class ContractOut(BaseModel):
    id: str
```

## Don't

```python
class Contract(BaseModel):
    id: str
```

## Suppression

Silence one line with `# botica: ignore[api-response-schema-name]` or `# noqa: BTDM008`.
