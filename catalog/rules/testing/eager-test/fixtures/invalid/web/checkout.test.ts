import { createCart, checkout, sendReceipt } from "./shop";

test("checkout", () => {
  const cart = createCart();
  cart.add(1);
  cart.remove(1);
  cart.applyCoupon("A");
  checkout(cart);
  sendReceipt(cart);
  expect(cart.total()).toBe(0);
});
