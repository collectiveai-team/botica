---
title: Trigger Prefect flows via run_deployment, not by importing flow functions
section: prefect
scope: specific
applies-to: prefect
status: current
tags: prefect, api, deployment, decoupling, orchestration
---

## Trigger Prefect flows via run_deployment, not by importing flow functions

The FastAPI app must never import flow functions directly. Instead it calls
`prefect.deployments.run_deployment` by deployment UUID, polls for the result via
`get_client`, and forces a `CRASHED` state with `anyio.move_on_after` if the run
exceeds the timeout. This keeps the API fully decoupled from flow internals: the
worker container owns the flow code; the API container only knows deployment IDs
and flow run IDs. `PREFECT_API_URL` / `PREFECT_BASE_URL` are injected via
`pydantic-settings` so the server address is configurable per environment.

**Prefer:**

```python
from prefect.deployments import run_deployment
from prefect.client.orchestration import get_client

# trigger — deployment_id is a UUID stored in config/DB, not an import
flow_run = await run_deployment(name=deployment_id, parameters=params, timeout=0)

# poll
async with get_client() as client:
    flow_run = await client.read_flow_run(flow_run.id)
    if flow_run.state.is_final():
        result = await flow_run.state.result()

# timeout + cleanup
async with get_client() as client:
    with anyio.move_on_after(timeout):
        while True:
            flow_run = await client.read_flow_run(flow_run_id)
            if flow_run.state.is_final():
                break
            await anyio.sleep(poll_interval)
    if not flow_run.state.is_final():
        await client.set_flow_run_state(flow_run_id, state="CRASHED", force=True)
```

**Avoid:**

```python
# importing and calling the flow function from the API — couples containers
from workflows.process_document.flow import process_document
result = await process_document(document_id=...)
```

Configure `PREFECT_BASE_URL` (e.g. `http://prefect-server:4200`) and `PREFECT_API_URL`
(e.g. `http://prefect-server:4200/api`) via `pydantic-settings`. Use a `field_validator`
to write `PREFECT_BASE_URL` into `os.environ` so the Prefect client picks it up
automatically.

Reference: see `prefect-serve-deploy` for how deployments are registered and
`prefect-webhook-hooks` for the alternative async notification path.
