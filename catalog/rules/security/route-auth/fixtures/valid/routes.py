from typing import Annotated

from fastapi import APIRouter, Depends, FastAPI, Security

from app import deps
from app.deps import CurrentAdmin, get_current_user
from app.models import Order, User

app = FastAPI()
router = APIRouter(prefix="/orders")
admin = APIRouter(prefix="/admin", dependencies=[Depends(get_current_user)])

CurrentUser = Annotated[User, Depends(get_current_user)]
type Caller = Annotated[User, Security(get_current_user, scopes=["orders"])]


@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


@router.get("/")
async def list_orders(user: Annotated[User, Depends(get_current_user)]) -> list[Order]:
    return []


@router.post("/")
async def create_order(user: CurrentUser) -> Order:
    return Order()


@router.put("/{order_id}")
async def replace_order(order_id: int, caller: Caller) -> Order:
    return Order()


@router.patch("/{order_id}", dependencies=[Depends(dependency=get_current_user)])
async def update_order(order_id: int) -> Order:
    return Order()


@router.get("/legacy")
def legacy_orders(user: User = Depends(get_current_user)) -> list[Order]:
    return []


@admin.delete("/orders/{order_id}")
async def delete_order(order_id: int) -> None:
    return None


@router.get("/admin")
async def admin_orders(user: CurrentAdmin) -> list[Order]:
    return []


@router.get("/audit")
async def audit(user: deps.CurrentAdmin) -> list[Order]:
    return []


def helper(value: int) -> int:
    return value
