import { execSync } from "node:child_process";

export function currentTag(): string {
  return execSync("git describe --tags").toString().trim();
}
