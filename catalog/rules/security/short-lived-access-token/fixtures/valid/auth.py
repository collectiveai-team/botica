from datetime import datetime, timedelta

import jwt
from pydantic import Field, SecretStr
from pydantic_settings import BaseSettings


class AuthSettings(BaseSettings):
    signing_key: SecretStr
    access_token_ttl_seconds: int = Field(default=900, gt=0)


def issue_access_token(user_id: str, settings: AuthSettings, now: datetime) -> str:
    expires = now + timedelta(seconds=settings.access_token_ttl_seconds)
    claims = {"sub": user_id, "iat": now, "exp": expires}
    return jwt.encode(claims, settings.signing_key.get_secret_value(), algorithm="HS256")
