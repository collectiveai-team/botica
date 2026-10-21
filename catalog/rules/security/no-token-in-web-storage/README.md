## Directive

Code that runs in the browser writes no credential to `localStorage` or `sessionStorage`. In
TypeScript and JavaScript (`.ts`, `.tsx`, `.js`), the rule reports `setItem` with a key that is a
string literal, and an assignment to a property or a literal index of the storage, when the key
holds `token`, `jwt`, `auth`, `session`, `secret`, `credential` or `password`, in any case. The
storage may be named as `localStorage`, `window.localStorage` or `globalThis.localStorage`, and the
same for `sessionStorage`. An error by default.

## Why

Every script of the origin can read web storage, so one injected script reads the token and sends
it away. The OWASP HTML5 Security Cheat Sheet says: "Do not store session identifiers in local
storage as the data is always accessible by JavaScript. Cookies can mitigate this risk using the
`httpOnly` flag." A cookie that the server sets with `HttpOnly` is out of reach of the scripts of
the page.

## Do

The server sets the session cookie, and the browser sends it with each request:

```ts
await fetch("/api/orders", { credentials: "include" });
```

## Don't

```ts
localStorage.setItem("accessToken", token);
sessionStorage.authToken = token;
```

## Suppression

One comment silences one line. It names the rule by its slug.

```ts
localStorage.setItem("sessionTheme", theme); // botica: ignore[no-token-in-web-storage]
```
