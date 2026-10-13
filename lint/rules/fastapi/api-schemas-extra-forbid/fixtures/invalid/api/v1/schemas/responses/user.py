from pydantic import BaseModel


class UserOut(BaseModel):
    id: int
