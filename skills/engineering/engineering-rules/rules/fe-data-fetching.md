---
title: Fetch backend data server-side; never expose secrets or backend URLs to the browser
section: fe
scope: specific
applies-to: nextjs
status: current
tags: nextjs, data-fetching, api, proxy, backend
---

## Fetch backend data server-side; never expose secrets or backend URLs to the browser

The recommended pattern is to fetch backend data on the server — either in a Server Component
or via a server-side Route Handler proxy — and keep credentials and internal URLs in
server-only environment variables. Do not expose `BACKEND_INTERNAL_URL` as a `NEXT_PUBLIC_`
variable and do not read auth tokens from `localStorage` in browser code.

**Recommended — catch-all proxy Route Handler**

All client requests go to `/api/proxy/*`. A server-side Route Handler
(`app/api/proxy/[...path]/route.ts`, `export const dynamic = "force-dynamic"`) forwards them
to `BACKEND_INTERNAL_URL` (server-only env var) and relays the response. Client code calls a
thin helper that injects `Authorization: Bearer` and hits the local proxy, never the backend
directly.

```ts
// lib/backend.ts — client-side helper; hits the local proxy, not the backend directly
export async function apiFetch<T>(path: string, token?: string, init?: RequestInit): Promise<T> {
  const url = `/api/proxy${path}`
  const headers: Record<string, string> = { "Content-Type": "application/json" }
  if (token) headers["Authorization"] = `Bearer ${token}`
  const res = await fetch(url, { ...init, headers })
  if (!res.ok) throw new Error(`${res.status}`)
  return res.json() as Promise<T>
}
```

**Also acceptable — Server Component fetch**

Fetch directly inside a Server Component using a server-only env var. The browser never sees
the URL or credentials.

```ts
// app/contracts/page.tsx — Server Component
export default async function ContractsPage() {
  const res = await fetch(`${process.env.BACKEND_INTERNAL_URL}/api/contracts`, {
    headers: { Authorization: `Bearer ${await getServerToken()}` },
  })
  const contracts = await res.json()
  return <ContractTable contracts={contracts} />
}
```

**Avoid:**

```ts
// Never expose internal backend URL to the browser
const BACKEND = process.env.NEXT_PUBLIC_BACKEND_INTERNAL_URL // leaks URL to client bundle

// Never read credentials from localStorage and call the backend directly from the browser
export async function authenticatedFetch(url: string, options: RequestInit = {}): Promise<Response> {
  const token = localStorage.getItem("access_token") // XSS-readable; avoid
  return fetch(url, { ...options, headers: { Authorization: `Bearer ${token}` } })
}
```

Reference: `fe-auth` for how tokens are obtained and forwarded.
