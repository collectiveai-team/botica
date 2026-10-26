import { getOrders } from "@/data/orders";
import { OrderFilter } from "./order-filter";

export default async function Page() {
  return <OrderFilter orders={await getOrders()} />;
}
