"""CES-79 · no raw dicts at boundaries — canonical boundary shapes.

Drop-in for a new repo, or a comparison template when aligning existing code to the house
pattern. A function that crosses a boundary returns a typed structure, never a raw ``dict``.

- Use a ``@dataclass`` for internal boundaries (no validation needed).
- Use a pydantic ``BaseModel`` where validation/serialization matters (e.g. API I/O).

See ``uvx botica explain no-dict-return-annotation`` (and ``uvx botica rules`` for its
siblings) for the full rule.
"""

from __future__ import annotations

from dataclasses import dataclass

from pydantic import BaseModel


# Internal boundary — a dataclass makes the contract explicit and type-checkable.
@dataclass
class Point:
    x: int
    y: int


def origin() -> Point:
    return Point(x=0, y=0)


# Validated boundary — a pydantic model where input/output must be validated.
class CreateUser(BaseModel):
    email: str
    display_name: str


def normalize(raw: CreateUser) -> CreateUser:
    return CreateUser(email=raw.email.lower(), display_name=raw.display_name.strip())


# Anti-pattern (flagged by CES-79) — do NOT do this at a boundary:
#
#     def origin() -> dict:            # no-dict-return-annotation
#         return {"x": 0, "y": 0}      # no-dict-literal-return
