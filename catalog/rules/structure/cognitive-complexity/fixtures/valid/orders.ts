export function total(order: Order): number {
  return order.items.filter(inStock).reduce((sum, item) => sum + item.price, 0);
}

function inStock(item: Item): boolean {
  return item.inStock && item.price > 0;
}
