---
title: Separate app construction, routers, and services; use lifespan for startup/shutdown
section: api
scope: specific
applies-to: fastapi
status: current
tags: fastapi, structure, lifespan, routers, services
---

## Separate app construction, routers, and services; use lifespan for startup/shutdown

`main.py` owns only three things: create the `FastAPI` instance, register middleware, and
wire routers via `include_router`. All startup/shutdown logic lives in a
`@asynccontextmanager lifespan` function — never in module-level side effects. Route
handlers are thin: they resolve dependencies and delegate to a service or CRUD layer;
no business logic sits inside them.

**Prefer:**

```python
# main.py — construction, middleware, router registration only
@asynccontextmanager
async def lifespan(app: FastAPI):
    # startup: run migrations, start scheduler …
    yield
    # shutdown: stop connector, flush resources …

app = FastAPI(title=settings.app_name, lifespan=lifespan)
app.add_middleware(CORSMiddleware, ...)
app.include_router(connectors.router, prefix=api_prefix)

# api/routes/connectors.py — thin handler
@router.post("/connectors/{id}/start")
async def start_connector(id: str, user: CurrentUser, db: DbSession):
    return await connector_manager.start(id)  # logic lives in services/
```

**Avoid:**

```python
# Don't put migration calls, scheduler starts, or business logic in main.py
# or directly inside route handlers.
@app.on_event("startup")          # deprecated; use lifespan instead
async def startup():
    run_migrations()
```

Reference: see `api-pydantic-settings` for `get_settings()` usage inside `main.py`.
