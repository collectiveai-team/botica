"use client";

import { useEffect, useState } from "react";

export default function Page() {
  const [orders, setOrders] = useState<string[]>([]);
  useEffect(() => {
    fetch("/api/orders").then((response) => response.json()).then(setOrders);
  }, []);
  return <ul>{orders.map((order) => <li key={order}>{order}</li>)}</ul>;
}
