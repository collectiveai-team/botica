import { expect, test } from "bun:test";

test("the status is clean in bun", async () => {
  const child = Bun.spawn(["git", "status", "--short"]);
  expect(await child.exited).toBe(0);
  expect(await Bun.$`git status --short`.text()).toBe("");
});

test("the status is clean in deno", async () => {
  const output = await new Deno.Command("git", { args: ["status"] }).output();
  expect(output.success).toBe(true);
});
