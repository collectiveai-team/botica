import jwt


def claims_of(token: str, key: str) -> dict[str, object]:
    return jwt.decode(token, key, algorithms=["HS256"], options={"require": ["exp"]})


def header_of(token: str) -> dict[str, object]:
    return jwt.get_unverified_header(token)


STRICT = {"verify_signature": True, "verify_exp": True}
