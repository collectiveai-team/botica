import { spawnSync } from "child_process";
import { render } from "@testing-library/react";
import { expect, test } from "vitest";

test("the version is shown", () => {
  const version = spawnSync("mytool", ["--version"]).stdout.toString();
  expect(render(<p>{version}</p>).container.textContent).toBe(version);
});
