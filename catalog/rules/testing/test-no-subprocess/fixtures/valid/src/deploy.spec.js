const { test } = require("node:test");
const assert = require("node:assert");

test("a file is read through the runtime", async () => {
  const file = Bun.file("package.json");
  assert.ok(await import("./deploy.js"));
  assert.ok(file);
});
