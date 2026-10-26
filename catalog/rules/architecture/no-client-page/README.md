## Directive

A `page` or a `layout` of the App Router of Next.js stays a Server Component, and the
`"use client"` directive goes on the interactive components that the page renders. The rule
reports the `"use client"` directive when it is the first statement of a file `page` or `layout`
(`.tsx`, `.ts`, `.jsx`, `.js`) under a folder `app`. A warning by default.

## Why

Next.js renders a file of the App Router as a Server Component unless it starts with
`"use client"`. The guide
[Server and Client Components](https://nextjs.org/docs/app/getting-started/server-and-client-components)
says that once a file is marked with `"use client"`, "all of its imports and the components it
directly renders are included in the client bundle". A page that is a Client Component cannot read
a database, a secret or a Data Access Layer while it renders, so it loads its data in the browser
after the first render. The same guide, in "Reducing JS bundle size", says: "add `'use client'` at
the top of the files that define your interactive components instead of marking large parts of
your UI as Client Components." The reference of
[`use client`](https://nextjs.org/docs/app/api-reference/directives/use-client) calls the file "an
entry point for the components to be rendered on the client side".

## Exceptions

A page that is interactive as a whole, such as an editor or a canvas with no data of the server,
keeps `"use client"` with a comment `botica: ignore-file[no-client-page] -- <reason>`.

## Do

```tsx
// app/orders/page.tsx
import { getOrders } from "@/data/orders";
import { OrderFilter } from "./order-filter"; // starts with "use client"

export default async function Page() {
  return <OrderFilter orders={await getOrders()} />;
}
```

## Don't

```tsx
// app/orders/page.tsx
"use client";

import { useState } from "react";

export default function Page() {
  const [filter, setFilter] = useState("");
  return <input value={filter} onChange={(event) => setFilter(event.target.value)} />;
}
```

## Suppression

One comment silences the file, with its reason. It names the rule by its slug.

```tsx
"use client";
// botica: ignore-file[no-client-page] -- the whole page is a drawing canvas
```
