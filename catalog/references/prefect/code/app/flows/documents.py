"""The flow that summarizes documents: thin orchestration over the domain code."""

from __future__ import annotations

import asyncio

from prefect import flow, get_run_logger, task
from prefect.artifacts import acreate_markdown_artifact

from app.core.documents import Document, Summary, summarize

# Lowercase letters, numbers and dashes only: Prefect rejects any other artifact key.
ARTIFACT_KEY = "document-summaries"


@task(retries=2, retry_delay_seconds=5)
async def summarize_document(document: Document) -> Summary:
    """Summarize one document, retried when it fails."""
    return summarize(document)


def _table(summaries: list[Summary]) -> str:
    rows = "".join(f"| {s.document_id} | {s.words} | {s.first_sentence} |\n" for s in summaries)
    return f"| document | words | first sentence |\n|---|---|---|\n{rows}"


@flow
async def summarize_documents(documents: list[Document]) -> list[Summary]:
    """Summarize the documents at the same time and publish the summaries as an artifact."""
    logger = get_run_logger()
    logger.info("summarizing %d documents", len(documents))
    async with asyncio.TaskGroup() as group:
        runs = [group.create_task(summarize_document(document)) for document in documents]
    summaries = [run.result() for run in runs]
    await acreate_markdown_artifact(
        markdown=_table(summaries),
        key=ARTIFACT_KEY,
        description="The summary of each document of the run",
    )
    return summaries
