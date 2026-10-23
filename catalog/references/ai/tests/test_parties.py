"""The agent of the topic, driven by a model function that stands for the provider."""

from __future__ import annotations

import asyncio
from typing import TYPE_CHECKING

from pydantic_ai.messages import (
    ModelRequest,
    ModelResponse,
    RetryPromptPart,
    ToolCallPart,
    UserPromptPart,
)
from pydantic_ai.models.function import AgentInfo, FunctionModel

from app.core.parties import Parties, Party, extract_parties, parties_agent
from app.core.prompts import load_prompt

if TYPE_CHECKING:
    from pydantic_ai.messages import ModelMessage

CONTRACT = "Ana Pérez sells to Acme S.A. the car described below."


def answer(arguments: dict[str, object]):
    def respond(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
        return ModelResponse(parts=[ToolCallPart(info.output_tools[0].name, arguments)])

    return respond


def test_the_prompt_comes_from_its_file():
    assert load_prompt("extract_parties").startswith("You read a contract and list its parties.")


def test_the_model_gets_the_prompt_as_instructions_and_the_contract_as_the_request():
    seen: list[ModelRequest] = []

    def respond(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
        request = messages[-1]
        assert isinstance(request, ModelRequest)
        seen.append(request)
        return answer({"parties": []})(messages, info)

    asyncio.run(extract_parties(parties_agent(FunctionModel(respond)), CONTRACT))

    assert seen[0].instructions == load_prompt("extract_parties").strip()
    prompts = [part.content for part in seen[0].parts if isinstance(part, UserPromptPart)]
    assert prompts == [CONTRACT]


def test_the_output_is_the_validated_model():
    parties = asyncio.run(
        extract_parties(
            parties_agent(
                FunctionModel(answer({"parties": [{"name": "Ana Pérez", "role": "seller"}]}))
            ),
            CONTRACT,
        )
    )

    assert parties == Parties(parties=[Party(name="Ana Pérez", role="seller")])


def test_an_output_that_fails_validation_is_sent_back_to_the_model():
    calls: list[list[ModelMessage]] = []

    def respond(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
        calls.append(messages)
        name = "" if len(calls) == 1 else "Acme S.A."
        return answer({"parties": [{"name": name, "role": "buyer"}]})(messages, info)

    parties = asyncio.run(extract_parties(parties_agent(FunctionModel(respond)), CONTRACT))

    assert parties.parties[0].name == "Acme S.A."
    retry = calls[1][-1]
    assert isinstance(retry, ModelRequest)
    assert any(isinstance(part, RetryPromptPart) for part in retry.parts)
