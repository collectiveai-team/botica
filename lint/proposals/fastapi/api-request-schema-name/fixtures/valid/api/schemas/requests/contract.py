from enum import Enum

from pydantic import BaseModel


class Kind(str, Enum):
    NDA = "nda"


class CreateContractRequest(BaseModel):
    name: str
    kind: Kind
