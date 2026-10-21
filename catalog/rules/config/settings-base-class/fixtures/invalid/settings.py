from pydantic import BaseModel


class Settings(BaseModel):
    database_url: str


class AppSettings(BaseModel):
    debug: bool = False
