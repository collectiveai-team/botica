---
id: CES-133
title: A Next.js app loads its data and reads its secrets on the server
---

## Directive

In an app of the App Router of Next.js, the server reads the secrets and loads the data, and the
browser gets the values it shows as props of a Client Component. A Client Component is a file
whose first statement is the `"use client"` directive.

- A Client Component reads only the environment variables whose name starts with `NEXT_PUBLIC_`,
  and `NODE_ENV`. A module that reads a secret imports `server-only`, so that the build fails
  when a Client Component imports it.
- A `page` and a `layout` stay Server Components. The `"use client"` directive goes on the
  interactive components that they render, the leaves of the tree. A page that is interactive as a
  whole, such as an editor or a canvas, keeps the directive with a suppression and its reason.
- A Server Component, a Data Access Layer that imports `server-only`, or a Route Handler loads the
  data, and the page passes it as props. A Client Component does not load in `useEffect` the data
  that the server could load, and calls no third-party API that needs a credential. It loads in
  the browser only what depends on the user after the first render: a search as the user types,
  infinite scroll, polling, a subscription, and the mutations of its forms.

## Why

Next.js renders a Client Component on the server too, to write the HTML of the first load, and
then again in the browser. The guide
[Data Security](https://nextjs.org/docs/app/guides/data-security) says that Client Components "run
on the server during prerendering, but must follow the same security assumptions as code running
in the browser". The server has every environment variable, so a private one that a Client
Component renders is written into the HTML that every visitor gets. Built with `next build`
16.4.2, a static page whose Client Component rendered `process.env.PROBE_SECRET_KEY` held its value in
`.next/server/app/index.html`. In the browser the same read is empty:
[Environment Variables](https://nextjs.org/docs/app/guides/environment-variables) says that a
variable without the prefix `NEXT_PUBLIC_` is "only available in the Node.js environment", and
that Next.js inlines the value of a variable with the prefix "into any JavaScript sent to the
browser". The same guide of data security says that "only the Data Access Layer should access
`process.env`", and that `server-only` causes "a build error if the module is imported in the
client environment".

The guide
[Server and Client Components](https://nextjs.org/docs/app/getting-started/server-and-client-components)
says that once a file is marked with `"use client"`, "all of its imports and the components it
directly renders are included in the client bundle". The reference of
[`use client`](https://nextjs.org/docs/app/api-reference/directives/use-client) calls the file "an
entry point for the components to be rendered on the client side". On a page or a layout the
directive moves the whole segment to the browser, where the page can no longer read a database or
a secret while it renders. The same guide, in "Reducing JS bundle size", says: "add `'use client'`
at the top of the files that define your interactive components instead of marking large parts of
your UI as Client Components."

The same guide uses Server Components to "fetch data from databases or APIs close to the source"
and to "use API keys, tokens, and other secrets without exposing them to the client", and passes
the data to the Client Component as props. A Client Component that fetches in `useEffect` renders
empty, waits for its JavaScript, and then asks for the data: one more round trip after the first
paint. A call to a third-party API from the browser carries its credential to the browser.

## Examples

```tsx
// data/orders.ts
import "server-only";

export async function getOrders(): Promise<Order[]> {
  const response = await fetch("https://api.shop.example.com/orders", {
    headers: { authorization: `Bearer ${process.env.SHOP_API_KEY}` },
  });
  return response.json();
}
```

```tsx
// app/orders/page.tsx
import { getOrders } from "@/data/orders";
import { OrderList } from "./order-list";

export default async function Page() {
  return <OrderList orders={await getOrders()} />;
}
```

```tsx
// app/orders/order-list.tsx
"use client";

export function OrderList({ orders }: { orders: Order[] }) {
  return <ul>{orders.map((order) => <li key={order.id}>{order.total}</li>)}</ul>;
}
```

The page is a Server Component that loads the orders through a module of the server, which reads
the key. The Client Component gets the orders as props.

```tsx
// app/orders/page.tsx
"use client";

import { useEffect, useState } from "react";

export default function Page() {
  const [orders, setOrders] = useState<Order[]>([]);
  useEffect(() => {
    fetch("https://api.shop.example.com/orders", {
      headers: { authorization: `Bearer ${process.env.SHOP_API_KEY}` },
    })
      .then((response) => response.json())
      .then(setOrders);
  }, []);
  return <ul>{orders.map((order) => <li key={order.id}>{order.total}</li>)}</ul>;
}
```

This page is a Client Component. Its HTML holds an empty list, the key is empty in the browser, and
the request runs after the first paint. Renamed to `NEXT_PUBLIC_SHOP_API_KEY`, the key would work
and every visitor would read it in the JavaScript of the page.
