from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    database_url: str


class SettingsView(BaseModel):
    debug: bool
