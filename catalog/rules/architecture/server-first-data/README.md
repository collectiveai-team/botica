## Directive

In a Next.js app, a Client Component does not load the data that the server can load. A Server
Component, a Data Access Layer that imports `server-only` or a Route Handler loads it, and the
Client Component gets it as props. A Client Component calls no third-party API that needs a
credential.

## Why

A Client Component that fetches in `useEffect` renders empty, waits for its JavaScript to load, and
only then asks for the data: one more round trip after the first paint, and a loading state that
the page would not need. A call to a third-party API from the browser needs the credential in the
browser, where every visitor can read it. The guide
[Server and Client Components](https://nextjs.org/docs/app/getting-started/server-and-client-components)
uses Server Components to "fetch data from databases or APIs close to the source" and to "use API
keys, tokens, and other secrets without exposing them to the client", and passes the data to the
Client Component as props. The guide
[Data Security](https://nextjs.org/docs/app/guides/data-security) recommends a Data Access Layer
that runs only on the server for new projects.

## Flag

- A Client Component that fetches in `useEffect` on mount, with no input of the user, data that
  the page could load on the server.
- A Client Component that calls an API client or a query library on mount for data that does not
  change with what the user does.
- A Client Component that calls a third-party API with a key, a token or another credential.

## Do not flag

- A file that does not start with the `"use client"` directive.
- Data that depends on input of the user after the first render: a search as the user types, a
  filter, infinite scroll.
- Polling, a subscription or a socket.
- A mutation sent from a form or an event handler.
- A call to a Route Handler of the same app, which runs on the server.

## Confirm

Before reporting, name the data that the component loads and the Server Component, Data Access
Layer or Route Handler that could load it instead.

## Do

```tsx
// app/orders/page.tsx
import { getOrders } from "@/data/orders"; // imports "server-only"
import { OrderList } from "./order-list"; // starts with "use client"

export default async function Page() {
  return <OrderList orders={await getOrders()} />;
}
```

## Don't

```tsx
"use client";

import { useEffect, useState } from "react";

export function OrderList() {
  const [orders, setOrders] = useState<Order[]>([]);
  useEffect(() => {
    fetch("https://api.shop.example.com/orders", {
      headers: { authorization: `Bearer ${process.env.NEXT_PUBLIC_SHOP_KEY}` },
    })
      .then((response) => response.json())
      .then(setOrders);
  }, []);
  return <ul>{orders.map((order) => <li key={order.id}>{order.total}</li>)}</ul>;
}
```

## Suppression

A file that loads its data in the browser on purpose takes
`botica: ignore-file[server-first-data] -- <reason>` in a comment. A folder goes in the
`extend-ignores` of the rule.
