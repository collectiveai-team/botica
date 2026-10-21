import jwt


def claims_of(token: str) -> dict[str, object]:
    return jwt.decode(token, options={"verify_signature": False})


def expired_claims_of(token: str, key: str) -> dict[str, object]:
    return jwt.decode(token, key, algorithms=["HS256"], options=dict(verify_exp=False))
