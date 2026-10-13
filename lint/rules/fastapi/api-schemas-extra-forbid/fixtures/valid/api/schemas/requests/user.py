from pydantic import BaseModel, ConfigDict


class CreateUser(BaseModel):
    model_config = ConfigDict(extra="forbid")

    name: str
