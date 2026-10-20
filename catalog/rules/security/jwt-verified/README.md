## Directive

Code that decodes a token checks its signature and its expiry. The rule reports the calls and the
options of the JWT libraries that skip either check. An error by default.

| Language | Library | Reported |
|---|---|---|
| Python | PyJWT | `"verify_signature": False` or `"verify_exp": False` in a dict, or `verify_signature=False` or `verify_exp=False` as an argument |
| TypeScript and JavaScript (`.ts`, `.tsx`, `.js`) | `jsonwebtoken`, `jose` | `jwt.decode(...)`, `jsonwebtoken.decode(...)`, `decodeJwt(...)`, `ignoreExpiration: true`, `'none'` in `algorithms` |
| Go | `golang-jwt/jwt` v5 | `.ParseUnverified(...)`, `jwt.WithoutClaimsValidation()`, `jwt.UnsafeAllowNoneSignatureType` |
| Rust | `jsonwebtoken` | `insecure_decode(...)`, `insecure_decode_claims(...)`, `dangerous_insecure_decode(...)`, `.insecure_disable_signature_validation()`, `validate_exp = false` or `validate_exp: false` |

## Why

A token whose signature is not checked carries whatever claims its sender wrote, and one whose
expiry is not checked stays valid after it is revoked or stolen. Each library keeps both checks on
by default and documents the calls above as the ones that turn them off: PyJWT reads the claims
"without performing validation of the signature" when `verify_signature` is `False`, and the
README of `jsonwebtoken` says that `jwt.decode` "will not verify whether the signature is valid".

## Do

### Python

```python
claims = jwt.decode(token, settings.jwt_key.get_secret_value(), algorithms=["HS256"])
```

### TypeScript

```ts
const claims = jwt.verify(token, publicKey, { algorithms: ["RS256"] });
```

### Go

```go
token, err := jwt.Parse(raw, keyFunc, jwt.WithValidMethods([]string{"RS256"}))
```

### Rust

```rust
let data = decode::<Claims>(&token, &key, &Validation::new(Algorithm::RS256))?;
```

## Don't

### Python

```python
claims = jwt.decode(token, options={"verify_signature": False})
```

### TypeScript

```ts
const claims = jwt.decode(token);
```

### Go

```go
token, _, err := jwt.NewParser().ParseUnverified(raw, &claims)
```

### Rust

```rust
let data = dangerous::insecure_decode::<Claims>(&token)?;
```

## Suppression

A comment on the line silences it, with the reason the token is trusted. In Python, `# botica:
ignore[jwt-verified]` or `# noqa: BTSEC002 -- <reason>`; in TypeScript, JavaScript, Go and Rust,
`// botica: ignore[jwt-verified]`.
