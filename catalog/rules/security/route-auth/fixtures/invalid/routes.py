from typing import Annotated

from fastapi import APIRouter, Depends, FastAPI, Header

from app.deps import get_session
from app.models import Order

app = FastAPI()
router = APIRouter(prefix="/orders", dependencies=[Depends(get_session)])


@router.get("/")
async def list_orders(user_id: str = Header()) -> list[Order]:
    return []


@router.post("/")
async def create_order(session: Annotated[object, Depends(get_session)]) -> Order:
    return Order()


@app.api_route("/export", methods=["GET", "POST"])
def export() -> bytes:
    return b""


@app.websocket("/live")
async def live(socket: object) -> None:
    return None
