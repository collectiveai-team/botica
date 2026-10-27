## Directive

A Client Component of Next.js reads only the environment variables whose name starts with
`NEXT_PUBLIC_`, and `NODE_ENV`. In TypeScript and JavaScript (`.ts`, `.tsx`, `.js`, `.jsx`), the
rule reports each `process.env.<NAME>` of a file whose first statement is the `"use client"`
directive, when `<NAME>` is neither `NODE_ENV` nor a name with the prefix `NEXT_PUBLIC_`. A
`"use client"` that is not the first statement, or that is only in a comment, does not make the
file a Client Component, and the rule leaves it alone. An error by default.

## Why

Next.js renders a Client Component on the server too, and puts the result in the HTML of the page.
The guide [Data Security](https://nextjs.org/docs/app/guides/data-security) says that Client
Components "run on the server during prerendering, but must follow the same security assumptions
as code running in the browser". The server has every variable, so a private one that the
component renders ends up in the HTML that every visitor gets: built with `next build` 16.4.2, a
page whose Client Component rendered `process.env.API_SECRET` held the secret in
`.next/server/app/index.html`. In the browser the same read is empty, since
[Environment Variables](https://nextjs.org/docs/app/guides/environment-variables) says that a
variable without the prefix `NEXT_PUBLIC_` is "only available in the Node.js environment". The
same guide of data security says that "only the Data Access Layer should access `process.env`", in
modules that import `server-only`, which fail the build when a Client Component imports them.

## Do

The page, a Server Component, reads the secret, and the Client Component gets only the value it
shows:

```tsx
// app/page.tsx
import { getPlan } from "@/lib/billing"; // imports "server-only" and reads process.env.BILLING_KEY

export default async function Page() {
  return <PlanBadge plan={await getPlan()} />;
}
```

```tsx
// app/plan-badge.tsx
"use client";

export function PlanBadge({ plan }: { plan: string }) {
  return <span data-site={process.env.NEXT_PUBLIC_SITE_NAME}>{plan}</span>;
}
```

## Don't

```tsx
"use client";

export function PlanBadge() {
  const key = process.env.BILLING_KEY;
  return <span>{key}</span>;
}
```

## Suppression

One comment silences one line. It names the rule by its slug.

```tsx
const region = process.env.DEPLOY_REGION; // botica: ignore[client-env-public-only]
```
