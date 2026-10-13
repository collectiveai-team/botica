---
title: Use a provider-backed session (NextAuth/Auth.js) with httpOnly cookies; avoid localStorage JWTs
section: fe
scope: specific
applies-to: nextjs
status: current
tags: nextjs, auth, nextauth, jwt, oauth, localstorage
---

## Use a provider-backed session (NextAuth/Auth.js) with httpOnly cookies; avoid localStorage JWTs

The recommended authentication pattern is NextAuth (Auth.js) with an OAuth provider. Tokens
are stored in httpOnly cookies, which are inaccessible to JavaScript. A middleware file gates
protected routes server-side before the page renders. This is the pattern to follow for new
projects.

**Recommended — NextAuth v4 + OAuth provider**

Configure `NextAuthOptions` with an OAuth provider. Use the `jwt` callback to capture the
provider's `id_token` and forward it to the session via the `session` callback. Extend
`Session` and `JWT` types via module augmentation. Use `withAuth` middleware to gate
protected routes server-side.

```ts
// lib/auth.ts
export const authOptions: NextAuthOptions = {
  providers: [GoogleProvider({ clientId: "...", clientSecret: "..." })],
  callbacks: {
    async jwt({ token, account }) {
      if (account?.id_token) token.idToken = account.id_token
      return token
    },
    async session({ session, token }) {
      session.idToken = token.idToken as string | undefined
      return session
    },
  },
}

// middleware.ts — guards routes server-side before render
export default withAuth(fn, { pages: { signIn: "/login" } })
export const config = { matcher: ["/dashboard", "/settings/:path*"] }
```

**Avoid — localStorage JWT**

Storing tokens in `localStorage` exposes them to any JavaScript running on the page,
including XSS payloads. Client-side redirect checks are also bypassable — they are not a
substitute for server-side route protection.

```ts
// lib/auth.ts — do not replicate this pattern
static getToken(): string | null {
  if (typeof window === "undefined") return null
  return localStorage.getItem("access_token") // readable by XSS
}
```

**Why httpOnly cookies are safer:** tokens in httpOnly cookies cannot be read by JavaScript
at all — only the browser's HTTP layer sends them with requests. NextAuth handles this
automatically. If you use a custom backend-issued JWT, set it as a `Secure; HttpOnly;
SameSite=Strict` cookie from a server action or API route rather than writing it to
`localStorage`.

Reference: `fe-data-fetching` for how tokens flow into API calls.
