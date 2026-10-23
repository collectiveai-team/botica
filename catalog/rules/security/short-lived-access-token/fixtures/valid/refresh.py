import hashlib
import secrets
from datetime import datetime, timedelta

REFRESH_TOKEN_TTL = timedelta(days=30)


def new_refresh_token(store: dict[str, datetime], now: datetime) -> str:
    token = secrets.token_urlsafe(32)
    store[hashlib.sha256(token.encode()).hexdigest()] = now + REFRESH_TOKEN_TTL
    return token
