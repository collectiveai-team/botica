import * as cp from "child_process";
import { promisify } from "node:util";
import { execa } from "execa";
import { expect, test } from "vitest";

const run = promisify(cp.exec);

test("the release is tagged", async () => {
  await run("git tag v1.0.0");
  const { stdout } = await execa("git", ["tag", "--list"]);
  expect(stdout).toContain("v1.0.0");
});
