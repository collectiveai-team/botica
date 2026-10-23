"""The summary of a document: free text from a model, whole or as it streams."""

from __future__ import annotations

from typing import TYPE_CHECKING

from pydantic_ai import Agent

from app.core.prompts import load_prompt

if TYPE_CHECKING:
    from collections.abc import AsyncIterator

    from pydantic_ai.models import Model


def summary_agent(model: Model | str) -> Agent[None, str]:
    """Return the agent that summarizes a document, with the model the settings name."""
    return Agent(model, instructions=load_prompt("summarize"), output_type=str)


async def summarize(agent: Agent[None, str], document: str) -> str:
    """Return the summary of a document once the model has written all of it."""
    result = await agent.run(document)
    return result.output


async def stream_summary(agent: Agent[None, str], document: str) -> AsyncIterator[str]:
    """Yield the summary of a document in pieces, as the model writes them."""
    async with agent.run_stream(document) as result:
        async for piece in result.stream_text(delta=True, debounce_by=None):
            yield piece
