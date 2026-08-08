---
title: Verify auth on every protected route — short-lived JWT or OAuth/OIDC ID token
section: api
scope: specific
applies-to: fastapi
status: current
tags: fastapi, auth, jwt, oauth, oidc, rbac, security
---

## Verify auth on every protected route — short-lived JWT or OAuth/OIDC ID token

The recommended default is short-lived JWTs (access token 15–30 min, refresh token
7 days) issued by your own service, verified on every protected request via a FastAPI
`Depends`. When you need SSO or third-party identity, verify the provider's ID token
server-side on every request instead of issuing your own token.

Long-lived tokens and client-supplied user identifiers are anti-patterns — do not use
them.

**Recommended — username/password → short-lived JWT**

```python
# auth.py — issue token
def create_access_token(subject: str, expires_delta: timedelta) -> str:
    payload = {
        "sub": str(subject),
        "exp": datetime.utcnow() + expires_delta,
        "iat": datetime.utcnow(),
    }
    return jwt.encode(payload, settings.SECRET_KEY.get_secret_value(), algorithm=settings.ALGORITHM)

# deps.py — verify on every protected route
reusable_oauth2 = OAuth2PasswordBearer(tokenUrl="/api/v0/login/access-token")

def get_current_user(db: Session = Depends(get_db_session), token: str = Depends(reusable_oauth2)) -> User:
    payload = jwt.decode(token, settings.SECRET_KEY.get_secret_value(), algorithms=[settings.ALGORITHM])
    user = db.exec(select(User).where(User.id == uuid.UUID(payload["sub"]))).first()
    if user is None:
        raise HTTPException(status_code=401, detail="User not found")
    return user

CurrentUser = Annotated[User, Depends(get_current_user)]
```

**Alternative — OAuth/OIDC provider ID token (e.g. Google, GitHub)**

Use this when your app delegates identity entirely to a third-party provider. Verify
the raw bearer token server-side; never trust a client-supplied identity claim.

```python
# auth.py — verify provider ID token on every request
from google.oauth2 import id_token
from google.auth.transport import requests as google_requests

def verify_google_id_token(raw_token: str) -> UserPrincipal:
    info = id_token.verify_oauth2_token(
        raw_token, google_requests.Request(), settings.google_client_id
    )
    # checks iss, email_verified, hosted_domain
    return UserPrincipal(email=info["email"], name=info.get("name"))

# deps.py — dependency with optional dev bypass
def get_current_user(authorization: str | None = Header(default=None)) -> UserPrincipal:
    if settings.disable_auth:
        return _dev_user()
    token = authorization.removeprefix("Bearer ").strip()
    return verify_google_id_token(token)

CurrentUser = Annotated[UserPrincipal, Depends(get_current_user)]

# rbac.py — role-gating dependency factory
def require_role(*allowed_roles: str) -> Depends:
    def dependency(user: CurrentUser, db: DbSession) -> UserPrincipal:
        role = db.exec(select(UserRole).where(UserRole.email == user.email)).first()
        if role.role == "admin" or role.role in allowed_roles:
            return user
        raise HTTPException(403, detail="Insufficient permissions")
    return Depends(dependency)
```

**Avoid:**

```python
# ✗ Never skip verification, trust a client-supplied user ID, or use long-lived tokens
@router.get("/admin")
def admin(user_id: str = Query(...)):   # ✗ unauthenticated
    ...

# ✗ Long-lived API keys stored in localStorage — no expiry, no revocation
token = jwt.encode({"sub": user_id}, SECRET, algorithm="HS256")  # no exp claim
```

Reference: `api-pydantic-settings` for `settings.SECRET_KEY` and `settings.ALGORITHM`.
