---
title: Deploy Prefect flows with prefect.serve in Python, not YAML
section: prefect
scope: specific
applies-to: prefect
status: current
tags: prefect, deployment, workflows, orchestration
---

## Deploy Prefect flows with prefect.serve in Python, not YAML

These workflows run on self-hosted Prefect 3.x. Deployments are declared in Python and served
from a dedicated worker entrypoint — there are no `prefect.yaml` deployment manifests. A
single `deploy.py` calls `prefect.serve(*deployments)` and that process is the worker
container. Keep new flows on this path so they register the same way and run under the same
worker.

**Prefer (Python deployments served from one entrypoint):**

```python
# workflows/deploy.py
from prefect import serve

process_document_deploy = process_document.to_deployment(name="process_document")
contract_comparison_deploy = contract_comparison.to_deployment(name="contract_comparison")

if __name__ == "__main__":
    serve(process_document_deploy, contract_comparison_deploy)
```

**Avoid:**

```yaml
# prefect.yaml — don't introduce a parallel deployment mechanism
deployments:
  - name: process_document
    entrypoint: workflows/process_document/flow.py:process_document
```

Reference: see `prefect-flows-tasks` for flow/task structure and `prefect-api-bridge` for how
the FastAPI app triggers these deployments via `run_deployment`.
