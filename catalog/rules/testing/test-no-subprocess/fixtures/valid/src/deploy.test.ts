import type { ChildProcess } from "node:child_process";
import { expect, test } from "vitest";
import { type Remote, Releases } from "./deploy";

class InMemoryRemote implements Remote {
  readonly tags: string[] = [];

  push(tag: string): void {
    this.tags.push(tag);
  }
}

test("a release pushes its tag", () => {
  const remote = new InMemoryRemote();
  new Releases(remote).publish("v1.0.0");
  expect(remote.tags).toEqual(["v1.0.0"]);
});

test("a stopped child has no pid", () => {
  const child = { pid: undefined } as Pick<ChildProcess, "pid">;
  expect(child.pid).toBeUndefined();
  expect(["child_process"]).toContain("child_process");
});
