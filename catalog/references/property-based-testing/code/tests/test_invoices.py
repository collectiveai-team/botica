"""The invoices, checked for any invoice that the strategies build."""

from __future__ import annotations

from hypothesis import given
from tests.strategies import invoices

from app.core.invoices import Invoice


@given(invoices)
def test_an_invoice_survives_a_round_trip_through_json(invoice: Invoice):
    assert Invoice.model_validate_json(invoice.model_dump_json()) == invoice


@given(invoices, invoices)
def test_the_total_of_two_invoices_joined_is_the_sum_of_their_totals(
    first: Invoice, second: Invoice
):
    joined = Invoice(lines=first.lines + second.lines)

    assert joined.total() == first.total() + second.total()
