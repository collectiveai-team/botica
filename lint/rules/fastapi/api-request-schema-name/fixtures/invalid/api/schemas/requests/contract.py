from pydantic import BaseModel


class Contract(BaseModel):
    name: str


class UpdateContractBody(BaseModel):
    name: str
