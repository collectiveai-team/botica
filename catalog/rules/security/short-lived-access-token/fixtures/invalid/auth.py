from datetime import datetime, timedelta

import jwt


def issue_access_token(user_id: str, key: str, now: datetime) -> str:
    claims = {"sub": user_id, "iat": now, "exp": now + timedelta(days=30)}
    return jwt.encode(claims, key, algorithm="HS256")
