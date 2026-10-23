import { execSync } from "node:child_process";
import { expect, test } from "vitest";

test("the tag is read", () => {
  expect(execSync("git describe").toString()).toMatch(/^v/);
});
