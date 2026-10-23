"""The lines of an invoice and its total."""

from __future__ import annotations

from decimal import Decimal

from pydantic import BaseModel, Field


class InvoiceLine(BaseModel):
    """A quantity of a product at a unit price."""

    product: str = Field(min_length=1)
    quantity: int = Field(gt=0)
    unit_price: Decimal = Field(ge=Decimal(0), decimal_places=2)


class Invoice(BaseModel):
    """An invoice has one line or more."""

    lines: list[InvoiceLine] = Field(min_length=1)

    def total(self) -> Decimal:
        """Return the sum of the quantity times the unit price of each line."""
        return sum((line.quantity * line.unit_price for line in self.lines), Decimal(0))
