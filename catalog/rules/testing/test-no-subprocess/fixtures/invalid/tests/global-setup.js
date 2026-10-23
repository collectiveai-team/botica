const { spawnSync } = require("child_process");

module.exports = async function globalSetup() {
  spawnSync("docker", ["compose", "up", "--wait"], { stdio: "inherit" });
};
