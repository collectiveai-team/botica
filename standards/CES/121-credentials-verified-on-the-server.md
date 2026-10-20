---
id: CES-121
title: Credentials are verified on the server
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

The lifetime of a token and the provider that issues it are choices of each project.

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
