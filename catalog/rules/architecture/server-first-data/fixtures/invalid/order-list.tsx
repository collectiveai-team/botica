"use client";

import { useEffect, useState } from "react";

type Order = { id: string; total: number };

export function OrderList() {
  const [orders, setOrders] = useState<Order[]>([]);
  useEffect(() => {
    fetch("https://api.shop.example.com/orders", {
      headers: { authorization: `Bearer ${process.env.NEXT_PUBLIC_SHOP_KEY}` },
    })
      .then((response) => response.json())
      .then(setOrders);
  }, []);
  return (
    <ul>
      {orders.map((order) => (
        <li key={order.id}>{order.total}</li>
      ))}
    </ul>
  );
}
