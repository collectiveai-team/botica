import pydantic
from fastapi import APIRouter

router = APIRouter()


class UserBody(pydantic.BaseModel):
    email: str
