---
title: Default to Server Components; push "use client" to leaf nodes (direction)
section: fe
scope: specific
applies-to: nextjs
status: current
tags: nextjs, server-components, use-client, rsc, spa
---

## Default to Server Components; push "use client" to leaf nodes (direction)

**Reality today:** many of these apps carry `"use client"` at line 1 of every `app/` page —
they behave as client-rendered SPAs that fetch a backend API over the network. No page-level
Server Components, no Server Actions, no `async/await` data fetching in the tree. This is a
common starting point, not the target.

**Direction for new and refactored code:** App Router defaults all components to Server
Components. Reserve `"use client"` for the smallest interactive leaf that actually needs it
(event handlers, browser APIs, hooks). Fetch data in Server Components; use Server Actions
for mutations.

**Prefer (target pattern):**

```tsx
// app/contracts/page.tsx — Server Component: no directive, async fetch
export default async function ContractsPage() {
  const contracts = await fetchContracts() // runs on the server
  return <ContractTable contracts={contracts} />
}

// components/contract-sort-button.tsx — only this leaf needs interactivity
"use client"
export function ContractSortButton({ onSort }: { onSort: () => void }) {
  return <button onClick={onSort}>Sort</button>
}
```

**Avoid (client-heavy SPA pattern to migrate away from):**

```tsx
// app/contracts/page.tsx — entire page as a client component
"use client"
export default function ContractsPage() {
  const [contracts, setContracts] = useState([])
  useEffect(() => { fetchContracts().then(setContracts) }, [])
  // ...
}
```

**Direction:** when touching a page, evaluate whether state/effects can move to a Server
Component. Pull `"use client"` down to the interactive subtree, not up to the route entry.
