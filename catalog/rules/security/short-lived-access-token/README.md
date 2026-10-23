## Directive

An access token that the server issues expires. Its lifetime is a number of seconds that the
settings read from `ACCESS_TOKEN_TTL_SECONDS`, after the prefix of the settings if they have one,
with a short default, such as 900, when the variable is not set. Each project sets its own value. A
session that lasts longer is kept by a refresh token that the server stores and replaces each time
it is used. A judgment rule, a warning, for Python, TypeScript, JavaScript, Go and Rust.

## Why

A signed access token is checked without a call to its issuer, so a stolen one works until it
expires, and one with no expiry works for ever. RFC 9700, section 4.14, says that refresh tokens
"allow the authorization server to issue access tokens with a short lifetime and reduced scope,
thus reducing the potential impact of access token leakage". A lifetime in the settings is changed
per deployment without a release. The lifetime reaches the call that signs the token through
settings and arguments, in the unit of each library, and the same call signs refresh tokens, so a
pattern cannot tell an access token of a month from a refresh token of a month.

## Flag

- An access token signed with no `exp`.
- An access token signed with a lifetime written in the code: `timedelta(days=30)`,
  `expiresIn: "7d"`, `setExpirationTime("1h")`, `time.Now().Add(24 * time.Hour)`.
- A setting of the lifetime that turns the expiry off, such as `None` or `0` meaning no expiry.

## Do not flag

- The value of the setting, or its default: each project picks it.
- A refresh token that the server stores and replaces each time it is used, whatever its lifetime.
- A file that only verifies tokens, such as the tokens of an identity provider.
- A token that the tests sign for themselves.

## Confirm

Follow the expiry from the call that signs the token to where its value comes from, and check from
the claims or the name of the function that the token authorizes requests on its own.

## Do

### Python

```python
class AuthSettings(BaseSettings):
    access_token_ttl_seconds: int = Field(default=900, gt=0)


def issue_access_token(user_id: str, settings: AuthSettings, now: datetime) -> str:
    expires = now + timedelta(seconds=settings.access_token_ttl_seconds)
    claims = {"sub": user_id, "iat": now, "exp": expires}
    return jwt.encode(claims, settings.signing_key.get_secret_value(), algorithm="HS256")
```

### TypeScript

```ts
return new SignJWT({ sub: userId })
  .setProtectedHeader({ alg: "HS256" })
  .setExpirationTime(now + settings.accessTokenTtlSeconds)
  .sign(settings.signingKey);
```

## Don't

### Python

```python
claims = {"sub": user_id, "exp": now + timedelta(days=30)}
```

### TypeScript

```ts
return jwt.sign({ sub: userId }, key, { expiresIn: "7d" });
```

## Suppression

The rule judges a file, so only the configuration silences it:
`ignore = ["short-lived-access-token"]`, or the paths in its `extend-ignores`.
