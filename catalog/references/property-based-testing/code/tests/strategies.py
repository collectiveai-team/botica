"""The strategies that build the models of the domain, with the constraints of their fields."""

from __future__ import annotations

from hypothesis import strategies as st

from app.core.invoices import Invoice, InvoiceLine

invoice_lines = st.builds(
    InvoiceLine,
    product=st.text(min_size=1),
    quantity=st.integers(min_value=1, max_value=10_000),
    unit_price=st.decimals(min_value=0, max_value=1_000_000, places=2),
)

invoices = st.builds(Invoice, lines=st.lists(invoice_lines, min_size=1, max_size=20))
