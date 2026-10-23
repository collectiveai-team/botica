from datetime import datetime

import jwt
from pydantic import SecretStr
from pydantic_settings import BaseSettings


class AuthSettings(BaseSettings):
    signing_key: SecretStr
    access_token_ttl_seconds: int = 900


def issue_access_token(user_id: str, settings: AuthSettings, now: datetime) -> str:
    claims = {"sub": user_id, "iat": now}
    return jwt.encode(claims, settings.signing_key.get_secret_value(), algorithm="HS256")
