"use client";

import { useEffect, useState } from "react";

type Order = { id: string; total: number };

export function OrderSearch({ orders }: { orders: Order[] }) {
  const [query, setQuery] = useState("");
  const [found, setFound] = useState<Order[]>(orders);
  useEffect(() => {
    if (query === "") {
      setFound(orders);
      return;
    }
    const controller = new AbortController();
    fetch(`/api/orders?q=${encodeURIComponent(query)}`, { signal: controller.signal })
      .then((response) => response.json())
      .then(setFound)
      .catch(() => {});
    return () => controller.abort();
  }, [query, orders]);
  return (
    <>
      <input value={query} onChange={(event) => setQuery(event.target.value)} />
      <ul>
        {found.map((order) => (
          <li key={order.id}>{order.total}</li>
        ))}
      </ul>
    </>
  );
}
