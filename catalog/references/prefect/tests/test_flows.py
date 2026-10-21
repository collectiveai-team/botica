"""The flow of the topic, run on the temporary server."""

from __future__ import annotations

import asyncio

from prefect.client.orchestration import get_client
from prefect.client.schemas.filters import (
    FlowRunFilter,
    FlowRunFilterName,
    LogFilter,
    LogFilterFlowRunId,
)
from prefect.client.schemas.sorting import FlowRunSort
from prefect.logging.handlers import APILogHandler

from app.core.documents import Document, Summary
from app.flows.documents import ARTIFACT_KEY, summarize_documents

DOCUMENTS = [
    Document(id="a", text="The lease starts in May. It lasts a year."),
    Document(id="b", text="The buyer pays on delivery."),
]


def test_the_flow_returns_a_summary_per_document():
    summaries = asyncio.run(summarize_documents(DOCUMENTS))

    assert summaries == [
        Summary(document_id="a", words=9, first_sentence="The lease starts in May"),
        Summary(document_id="b", words=5, first_sentence="The buyer pays on delivery"),
    ]


def test_the_flow_publishes_the_summaries_as_a_markdown_artifact():
    async def latest() -> str:
        await summarize_documents(DOCUMENTS)
        # The readers of artifacts of the client of prefect 3.8.8 send `flow_runs: null`, which its
        # server rejects with a 422, or an empty filter of task runs, which leaves out an artifact
        # of a flow. The request names the key only.
        async with get_client() as client:
            response = await client.request(
                "POST",
                "/artifacts/filter",
                json={
                    "artifacts": {"key": {"any_": [ARTIFACT_KEY]}},
                    "sort": "UPDATED_DESC",
                    "limit": 1,
                    "offset": 0,
                },
            )
        [artifact] = response.json()
        return str(artifact["data"])

    assert "| a | 9 | The lease starts in May |" in asyncio.run(latest())


def test_the_events_of_the_domain_reach_the_logs_of_the_run():
    async def messages() -> list[str]:
        await summarize_documents.with_options(flow_run_name="logs-of-the-domain")(DOCUMENTS)
        await APILogHandler.aflush()
        async with get_client() as client:
            [flow_run] = await client.read_flow_runs(
                flow_run_filter=FlowRunFilter(name=FlowRunFilterName(any_=["logs-of-the-domain"])),
                sort=FlowRunSort.START_TIME_DESC,
                limit=1,
            )
            logs = await client.read_logs(
                log_filter=LogFilter(flow_run_id=LogFilterFlowRunId(any_=[flow_run.id])),
                limit=200,
                offset=0,
            )
        return [log.message for log in logs]

    found = asyncio.run(messages())

    assert "summarizing 2 documents" in found
    assert any('"event": "document_summarized"' in message for message in found)
