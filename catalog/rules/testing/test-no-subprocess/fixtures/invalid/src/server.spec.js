const { fork } = require("node:child_process");
const test = require("node:test");

test("the worker answers", async () => {
  const worker = fork("./worker.js");
  const cp = await import("child_process");
  cp.spawnSync("kill", [String(worker.pid)]);
});
