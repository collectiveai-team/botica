export function total(order: Order): number {
  let result = 0;
  if (order.items) {
    for (const item of order.items) {
      if (item.inStock && item.price > 0) {
        result += item.price;
      }
    }
  }
  return result;
}
