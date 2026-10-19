from fastapi import APIRouter
from pydantic import BaseModel

router = APIRouter()


class CreateContract(BaseModel):
    name: str


@router.post("/contracts")
def create(body: CreateContract):
    return body
