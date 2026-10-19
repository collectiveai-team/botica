from pydantic import BaseModel


class CreateContractRequest(BaseModel):
    name: str
