import assert from "node:assert";

const total = (prices: number[]) => prices.reduce((a, b) => a + b, 0);

test("total", () => {
  expect(total([1, 2])).toBe(3);
  expect(total([1])).toEqual(1);
  assert.strictEqual(total([2]), 2);
  assert.ok(total([1]) > 0);
});
