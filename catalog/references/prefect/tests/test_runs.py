"""The API starts flow runs of the deployment, and a run that takes too long is cancelled."""

from __future__ import annotations

import asyncio
from uuid import UUID

import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient
from prefect.client.orchestration import get_client
from prefect.client.schemas.objects import StateType

from app.api.v1.routers.documents import router
from app.deploy import deployments
from app.runs import SUMMARIZE_DOCUMENTS, cancel, wait

pytestmark = pytest.mark.integration


def register() -> None:
    async def apply() -> None:
        for deployment in await deployments():
            await deployment.aapply()

    asyncio.run(apply())


def test_the_api_names_the_deployment_that_the_process_serves():
    [deployment] = asyncio.run(deployments())

    assert f"{deployment.flow_name}/{deployment.name}" == SUMMARIZE_DOCUMENTS


def test_the_api_starts_a_flow_run_of_the_deployment_and_returns_its_id():
    register()
    app = FastAPI()
    app.include_router(router, prefix="/v1")

    response = TestClient(app).post(
        "/v1/documents/summaries", json={"documents": [{"id": "a", "text": "One. Two."}]}
    )

    assert response.status_code == 202
    flow_run_id = UUID(response.json()["flow_run_id"])

    async def read() -> tuple[StateType | None, dict[str, object]]:
        async with get_client() as client:
            flow_run = await client.read_flow_run(flow_run_id)
        return flow_run.state_type, flow_run.parameters

    state, parameters = asyncio.run(read())
    assert state == StateType.SCHEDULED
    assert parameters == {"documents": [{"id": "a", "text": "One. Two."}]}


def test_a_flow_run_that_does_not_finish_in_time_is_cancelled():
    register()
    app = FastAPI()
    app.include_router(router)
    response = TestClient(app).post(
        "/documents/summaries", json={"documents": [{"id": "a", "text": "One."}]}
    )
    flow_run_id = UUID(response.json()["flow_run_id"])

    async def wait_or_cancel() -> StateType | None:
        try:
            async with asyncio.timeout(1):
                return await wait(flow_run_id)
        except TimeoutError:
            return await cancel(flow_run_id)

    state = asyncio.run(wait_or_cancel())

    assert state in {StateType.CANCELLING, StateType.CANCELLED}
