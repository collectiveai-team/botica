import { createCart } from "./shop";

test("add", () => {
  const cart = createCart();
  cart.add(1);
  expect(cart.items()).toHaveLength(1);
  expect(JSON.stringify(cart.items())).toBe("[1]");
});
