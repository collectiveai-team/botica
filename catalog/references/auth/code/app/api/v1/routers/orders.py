"""The endpoints of orders: one for the caller of an access token, one public."""

from fastapi import APIRouter

from app.api.dependencies import CurrentUser
from app.api.v1.schemas.responses.orders import OrdersResponse

router = APIRouter(tags=["orders"])


@router.get("/orders")
def list_orders(user: CurrentUser) -> OrdersResponse:
    """Return the orders of the caller."""
    return OrdersResponse(user_id=user, orders=[])


@router.get("/health")
def health() -> None:
    """Answer 200 to anyone: a public route."""
