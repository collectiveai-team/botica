import { $ } from "zx";
import spawn from "cross-spawn";
import { expect, test } from "vitest";

test("the build prints its version", async () => {
  expect((await $`node build.mjs --version`).stdout).toMatch(/\d/);
  expect(spawn.sync("node", ["build.mjs"]).status).toBe(0);
});
