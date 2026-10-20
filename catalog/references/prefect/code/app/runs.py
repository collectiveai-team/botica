"""The flow runs of a deployment, started and awaited from another process, such as the API.

The module names the deployment and imports no flow, so the API does not load the code of the
flows nor their dependencies.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from prefect.client.orchestration import get_client
from prefect.deployments import arun_deployment
from prefect.exceptions import FlowRunWaitTimeout
from prefect.flow_runs import wait_for_flow_run
from prefect.states import Cancelling

if TYPE_CHECKING:
    from uuid import UUID

    from prefect.client.schemas.objects import StateType

    from app.core.documents import Document

# `<flow name>/<deployment name>`, as `app/deploy.py` serves it.
SUMMARIZE_DOCUMENTS = "summarize-documents/summarize-documents"


async def start_summaries(documents: list[Document]) -> UUID:
    """Create a flow run of the deployment and return its id without waiting for it."""
    flow_run = await arun_deployment(
        name=SUMMARIZE_DOCUMENTS,
        parameters={"documents": [document.model_dump() for document in documents]},
        timeout=0,
    )
    return flow_run.id


async def wait_or_cancel(flow_run_id: UUID, timeout: int) -> StateType | None:
    """Wait for a flow run up to `timeout` seconds; past it, cancel the run."""
    try:
        flow_run = await wait_for_flow_run(flow_run_id, timeout=timeout)
    except FlowRunWaitTimeout:
        async with get_client() as client:
            await client.set_flow_run_state(flow_run_id, state=Cancelling(), force=True)
            flow_run = await client.read_flow_run(flow_run_id)
    return flow_run.state_type
