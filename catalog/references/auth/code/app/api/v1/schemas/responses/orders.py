"""The response schemas of the endpoints of orders."""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict


class OrdersResponse(BaseModel):
    """The orders of the caller."""

    model_config = ConfigDict(extra="forbid")

    user_id: str
    orders: list[str]
