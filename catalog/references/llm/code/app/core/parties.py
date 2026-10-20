"""The parties of a contract, extracted by a model into a validated structure."""

from __future__ import annotations

from typing import TYPE_CHECKING

from pydantic import BaseModel, Field
from pydantic_ai import Agent

from app.core.prompts import load_prompt

if TYPE_CHECKING:
    from pydantic_ai.models import Model


class Party(BaseModel):
    """A person or an organization bound by the contract."""

    name: str = Field(min_length=1)
    role: str = Field(min_length=1)


class Parties(BaseModel):
    """The parties of one contract."""

    parties: list[Party]


def parties_agent(model: Model | str) -> Agent[None, Parties]:
    """Return the agent that extracts the parties, with the model the settings name."""
    return Agent(
        model,
        instructions=load_prompt("extract_parties"),
        output_type=Parties,
        retries=2,
    )


async def extract_parties(agent: Agent[None, Parties], contract: str) -> Parties:
    """Return the parties of the text of a contract."""
    result = await agent.run(contract)
    return result.output
