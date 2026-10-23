The server verifies each caller from a signed access token that expires, and keeps a longer
session with a refresh token that it stores as a hash and replaces at each use. The same design is
shown four times: an API of FastAPI with PyJWT in `code/app/`, route handlers of Next.js with
`jose` in `code/web/`, a server of `net/http` with `golang-jwt` in `code/go/`, and a library with
`jsonwebtoken` in `code/rust/`.

## What to read

- The settings hold the key, the issuer, the audience and the two lifetimes, in seconds. The
  access token lives `APP_AUTH_ACCESS_TOKEN_TTL_SECONDS` seconds, 900 when the variable is not set,
  and the refresh token `APP_AUTH_REFRESH_TOKEN_TTL_SECONDS`, 14 days when it is not set. A
  lifetime that is not a number above 0 is an error; there is no upper limit, since each project
  sets its own value. `AuthSettings` in Python, `loadSettings` in TypeScript, `LoadSettings` in Go
  and `Settings::load` in Rust.
- The access token carries `sub`, `iss`, `aud`, `iat` and `exp`, with `exp` at `iat` plus the
  lifetime of the settings. Its verification names the one algorithm, the issuer and the audience,
  and requires `exp`: `jwt.decode(..., algorithms=[...], audience=..., issuer=...,
  options={"require": [...]})`, `jwtVerify(..., { algorithms, issuer, audience, requiredClaims })`,
  `jwt.ParseWithClaims(..., jwt.WithValidMethods(...), jwt.WithExpirationRequired())` and
  `Validation::new(Algorithm::HS256)` with `set_issuer`, `set_audience` and
  `set_required_spec_claims`.
- `Sessions` keeps the refresh tokens behind the port `RefreshTokenStore`. It stores the SHA-256
  of each token and never the token. `rotate` checks that the token is live (`is_live`), marks it
  as used through the `mark_used` step of the port, and returns a new one of the same session; a token that comes back after it was used deletes every token of its session, since
  the caller or a thief holds a copy. `close` ends the session at sign-out.
- Python has two adapters of the port. `InMemoryRefreshTokenStore` keeps the tokens in a dict, for
  one process and for the tests. `SqlRefreshTokenStore` keeps them in the table `refresh_tokens`,
  with the layout of the topic `database`: the row in `app/database/models/`, the adapter in
  `app/database/repositories/`, the table created by a migration of Alembic, and
  `get_sessions` wiring it over the session of each request. Its `mark_used` is one
  `UPDATE ... WHERE used IS false`, so of two requests that send the same token at once, one
  rotates it and the other ends the session. The tests run the same cases on both adapters, the SQL
  one on SQLite in memory built by the migrations.
- The refresh token travels in its own cookie, `HttpOnly`, `Secure` and `SameSite=Strict`, with
  the path of the routes that refresh and sign out, so the browser sends it nowhere else.
- `POST /v1/auth/session` and `POST /v1/browser/session` of Python open a session for the user of
  a token of the identity provider. `verify_provider_token` checks it with the keys the provider
  publishes, found by `PyJWKClient`, and with the issuer and the audience of the settings.
- In Next.js both tokens go in cookies, and JavaScript of the page reads neither. In Go,
  `RequireUser` wraps the handlers. Rust shows the settings, the tokens and the sessions, and no
  web framework.
- The tests sign tokens with the key of the settings, move the time of the refresh tokens with a
  clock of the test, and send the `Secure` cookies by hand, since the test clients speak plain
  HTTP.

## The access token in the body or in a cookie

The Python API shows both, and `get_current_user` takes the token from either: the header
`Authorization: Bearer`, or the cookie `__Host-access`.

- In the body (`app/api/v1/routers/auth.py`): `POST /v1/auth/session` and `/v1/auth/refresh`
  return `access_token` and `expires_in`, and the client sends the token as `Bearer`. It fits a
  client that is not a browser, such as a mobile application, a command-line tool or another
  service, and a page that calls an API of another site. The cost is that the code of the client
  holds the token: in a page, a script injected into it can read it while it lives, so the page
  keeps it in memory only and never in `localStorage` (rule `no-token-in-web-storage`).
- In cookies (`app/api/v1/routers/browser_session.py`): `POST /v1/browser/session` and
  `/v1/browser/refresh` answer 204 and set `__Host-access`, `HttpOnly` with `Path=/`, next to the
  refresh cookie. No script of the page reads either token, as in the cookie-based session of
  RFC 10017, section 6.1. The cost is that the browser sends the cookie by itself on every request
  to the origin, so the API depends on `SameSite=Strict` against cross-site requests and on the
  page and the API sharing a site; a page of another site cannot use it, and neither can a client
  that is not a browser.

## What to adapt

- The sign-in is the part of the identity provider. Use a library of OAuth 2.0 and OpenID Connect
  for the authorization code flow with PKCE (Authlib in Python, Auth.js or `openid-client` in
  Next.js, `golang.org/x/oauth2` with `coreos/go-oidc` in Go, `openidconnect` in Rust), and call
  the function that opens a session when the provider returns the user: `sign_in`,
  `browser_sign_in`, `openSession`, `Sessions.Open` or `Sessions::open`.
- When every caller holds tokens of the provider, the server can verify them on each request with
  `verify_provider_token` and issue none of its own.
- Keep one of the two routers of Python when the project has one kind of client.
- `InMemoryRefreshTokenStore` in Python and in TypeScript, `MemoryStore` in Go and in Rust hold
  the tokens of one process. With several processes, use `SqlRefreshTokenStore`, or implement the
  port over the database of the project in the other languages, with `markUsed`, `MarkUsed` or
  `mark_used` as one conditional update. Add the table to the migrations of the project (topic
  `database`).
- HS256 with one key fits a service that signs and verifies its own tokens. When other services
  verify them, sign with a private key (RS256 or ES256) and publish the public one.
- List `get_current_user` and `CurrentUser` in `dependencies` of `[rules.route-auth]`, and the
  routes of the session and of health in `public`.
- The lifetimes are the defaults of the references: 900 seconds for the access token, 14 days for
  the refresh token. Set yours in the environment.
