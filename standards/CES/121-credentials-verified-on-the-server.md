---
id: CES-121
title: Credentials are verified on the server and expire soon
references: ["auth"]
---

## Directive

The server takes the identity of a caller from a credential it verifies, and from nothing else the
client sends.

- Each route that is not declared public gets its caller from a dependency that verifies a signed
  credential with an expiry. A user id in a header, a query parameter or the body is not a
  credential.
- Code that decodes a token checks its signature and its expiry. A decode that skips either one is
  for a token that the code trusts already, and the line says why.
- Code that runs in the browser keeps no credential in `localStorage` or `sessionStorage`. The
  server sets the session in a cookie that is `HttpOnly`, `Secure` and `SameSite`.
- An access token that the server issues expires. Its lifetime is a number of seconds that the
  settings read from the environment variable `ACCESS_TOKEN_TTL_SECONDS`, after the prefix of the
  settings of the project if they have one (`APP_ACCESS_TOKEN_TTL_SECONDS`), with a short default
  when the variable is not set; the references of botica use 900, 15 minutes. Each project sets its
  own value, and no value turns the expiry off.
- A session that lasts longer is kept by a refresh token: a random value that the server stores as
  a hash, replaces with a new one each time it is used and deletes at logout. A refresh token that
  is used a second time ends the session it belongs to. The browser holds it in an `HttpOnly`
  cookie of its own, sent only to the route that refreshes.
- Signing users in through an identity provider, with OAuth 2.0 and OpenID Connect, the
  authorization code flow and PKCE, through a library, is preferred to a login that the service
  builds itself. When the project uses a provider, the server verifies the tokens of the provider
  with the keys that the provider publishes, and checks their issuer and their audience.

## Why

A route with no check of the caller answers anyone who can reach it. When a new route is added
without its dependency, nothing in the code shows that it is missing, so the rule lists the
dependencies that verify a caller and reports the routes that have none.

A token whose signature is not checked carries any claims its sender wrote, and one whose expiry is
not checked stays valid after it is revoked or stolen. The libraries keep these checks on by default
and turn them off with an option or a function of their own, such as `options={"verify_signature":
False}` in PyJWT, `jwt.decode` in `jsonwebtoken` or `ParseUnverified` in `golang-jwt`.

Every script of the origin can read web storage. The OWASP HTML5 Security Cheat Sheet says: "Do not
store session identifiers in local storage as the data is always accessible by JavaScript. Cookies
can mitigate this risk using the `httpOnly` flag."

A server checks a signed access token without asking anyone, so it cannot take back one that was
stolen: the token works until it expires. The OAuth 2.0 Security Best Current Practice (RFC 9700,
section 4.14) says that refresh tokens "allow the authorization server to issue access tokens with
a short lifetime and reduced scope, thus reducing the potential impact of access token leakage",
and that a refresh token is protected by rotation: "the authorization server issues a new refresh
token with every access token refresh response. The previous refresh token is invalidated". The
OWASP JSON Web Token Cheat Sheet says: "Token reuse can be mitigated by using short expiration time
in the JWT." Neither text gives a number. Fifteen minutes is the lower end of the idle timeouts
that the OWASP Session Management Cheat Sheet gives for applications of low risk, "15-30
minutes", and above the "2-5 minutes" it gives for those of high value.

The lifetime is in seconds because the claim `exp` of a JWT is a number of seconds (RFC 7519,
section 2, "NumericDate") and so is `expires_in` in the token response of OAuth 2.0 (RFC 6749,
section 5.1). One name in every language lets a deployment set the lifetime of a service without
reading its code, and a lifetime in the settings changes per environment without a release.

A token that the service signs for itself duplicates what an identity provider already does, with
its own key to keep and its own revocation to build. OAuth 2.0 for Browser-Based Applications
(RFC 10017, section 6.1) keeps the tokens of the provider on the server, in a cookie-based session,
"avoiding the direct exposure of any tokens to the browser-based application".

RFC 9700 (section 2.2.1) also says that servers "SHOULD use mechanisms for sender-constraining
access tokens, such as mutual TLS for OAuth 2.0 [RFC8705] or OAuth 2.0 Demonstrating Proof of
Possession (DPoP) [RFC9449]". In the browser of this standard no script can read the token, which
travels in an `HttpOnly` cookie. A client that holds tokens itself, such as a mobile application
or a command-line tool, can bind them to its key with DPoP or mutual TLS when the provider supports
it; the standard does not require it.

## Examples

```python
CurrentUser = Annotated[User, Depends(get_current_user)]


@router.get("/orders")
async def list_orders(user: CurrentUser) -> list[Order]: ...
```

```python
@router.get("/orders")
async def list_orders(user_id: str = Header()) -> list[Order]: ...
```

The first route verifies the caller through `get_current_user`; the second trusts a header.

```python
class AuthSettings(BaseSettings):
    access_token_ttl_seconds: int = Field(default=900, gt=0)


payload = {"sub": user_id, "exp": now + timedelta(seconds=settings.access_token_ttl_seconds)}
```

```python
payload = {"sub": user_id, "exp": now + timedelta(days=30)}
```

The first access token lives 900 seconds unless `ACCESS_TOKEN_TTL_SECONDS` sets another number; the
second lives a month, written in the code, and is valid for that month after it leaks.
