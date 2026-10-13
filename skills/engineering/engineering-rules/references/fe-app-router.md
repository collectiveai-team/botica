---
title: Use App Router exclusively; co-locate page components with their route segment
section: fe
scope: specific
applies-to: nextjs
status: current
tags: nextjs, app-router, routing, layout
---

## Use App Router exclusively; co-locate page components with their route segment

Use the Next.js App Router with no Pages Router remnant. Each route segment owns its own
`page.tsx` and `layout.tsx`; shared UI components live separately in `components/`. New
routes must follow this shape — never introduce `pages/`.

**Prefer:**

```tsx
// app/settings/layout.tsx — layout wraps the segment
export default function SettingsLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-background">
      <Navbar />
      <main className="px-6 py-8">{children}</main>
    </div>
  )
}

// app/page.tsx — redirect-only root; no UI here
import { redirect } from "next/navigation"
export default function Home() { redirect("/chat") }
```

**Avoid:**

```tsx
// pages/chat.tsx — Pages Router; not used, don't introduce it
// app/chat/ChatPage.tsx — page component co-located but named wrong;
//   route entry point must be page.tsx, not an arbitrary file
```

Reference: `fe-server-components-direction` for the `"use client"` story inside these routes.
