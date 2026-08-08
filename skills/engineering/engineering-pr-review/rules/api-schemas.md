---
title: Use separate request and response Pydantic models; never reuse one for both
section: api
scope: specific
applies-to: fastapi
status: current
tags: fastapi, pydantic, schemas, request, response, validation
---

## Use separate request and response Pydantic models; never reuse one for both

Request schemas and response schemas are distinct classes. A request model can never
double as a response model and vice versa. Put `ConfigDict(extra="forbid")` on request
schemas to reject unknown fields. Put `ConfigDict(from_attributes=True)` on response
schemas that are populated from ORM objects. All schema classes live in a dedicated
`schemas.py` module — not inline in route files.

**Prefer:**

```python
# Request — forbid extra fields to catch client mistakes early
class CreateDashboardRequest(BaseModel):
    model_config = ConfigDict(extra="forbid")
    name: str
    category: str = Field(pattern=r"^(finance|recruiting|marketing|general)$")

# Response — from_attributes=True for ORM-to-schema conversion
class DashboardOut(BaseModel):
    model_config = ConfigDict(extra="forbid")
    id: str
    name: str
    created_by: str
    created_at: datetime

# Conversion at the route boundary
return DashboardOut.model_validate(db_obj, from_attributes=True)
```

**Avoid:**

```python
# ✗ Reusing one model for both directions leaks internal fields and loses validation
class Dashboard(SQLModel, table=True):
    ...

@router.post("/dashboards", response_model=Dashboard)
def create(req: Dashboard): ...
```

Follow the `{Name}Request` / `{Name}Out` naming convention and always add
`ConfigDict(extra="forbid")` to request models. Response-only models (e.g. `Token`,
`TokenPayload`, `UserPublic`) should be kept in their own files, separate from
request schemas.
