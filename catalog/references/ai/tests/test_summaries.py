"""The free-text agent of the topic, whole and streamed, driven by a model function."""

from __future__ import annotations

import asyncio
from typing import TYPE_CHECKING

from pydantic_ai.messages import ModelRequest, ModelResponse, TextPart
from pydantic_ai.models.function import AgentInfo, FunctionModel

from app.core.prompts import load_prompt
from app.core.summaries import stream_summary, summarize, summary_agent

if TYPE_CHECKING:
    from collections.abc import AsyncIterator

    from pydantic_ai.messages import ModelMessage

DOCUMENT = "Acme S.A. asks the city for a permit to build a warehouse of 2,000 m2."
SUMMARY = "Acme S.A. asks the city for a permit for a warehouse of 2,000 m2."


def write(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    return ModelResponse(parts=[TextPart(SUMMARY)])


async def write_in_pieces(messages: list[ModelMessage], info: AgentInfo) -> AsyncIterator[str]:
    for piece in ("Acme S.A. asks the city ", "for a permit ", "for a warehouse of 2,000 m2."):
        yield piece


async def collect(pieces: AsyncIterator[str]) -> list[str]:
    return [piece async for piece in pieces]


def test_the_model_gets_the_prompt_as_instructions():
    seen: list[ModelRequest] = []

    def respond(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
        request = messages[-1]
        assert isinstance(request, ModelRequest)
        seen.append(request)
        return write(messages, info)

    asyncio.run(summarize(summary_agent(FunctionModel(respond)), DOCUMENT))

    assert seen[0].instructions == load_prompt("summarize").strip()


def test_the_output_is_the_text_of_the_model():
    assert asyncio.run(summarize(summary_agent(FunctionModel(write)), DOCUMENT)) == SUMMARY


def test_the_stream_yields_the_text_in_the_pieces_the_model_writes():
    agent = summary_agent(FunctionModel(stream_function=write_in_pieces))

    pieces = asyncio.run(collect(stream_summary(agent, DOCUMENT)))

    assert pieces == ["Acme S.A. asks the city ", "for a permit ", "for a warehouse of 2,000 m2."]
    assert "".join(pieces) == SUMMARY
