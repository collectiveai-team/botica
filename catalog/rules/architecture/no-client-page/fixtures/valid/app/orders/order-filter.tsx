"use client";

import { useState } from "react";

export function OrderFilter({ orders }: { orders: string[] }) {
  const [filter, setFilter] = useState("");
  return (
    <>
      <input value={filter} onChange={(event) => setFilter(event.target.value)} />
      <ul>{orders.filter((order) => order.includes(filter)).map((order) => <li key={order}>{order}</li>)}</ul>
    </>
  );
}
