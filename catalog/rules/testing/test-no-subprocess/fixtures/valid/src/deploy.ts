import { execFileSync } from "node:child_process";

export interface Remote {
  push(tag: string): void;
}

export class GitRemote implements Remote {
  push(tag: string): void {
    execFileSync("git", ["push", "origin", tag]);
  }
}

export class Releases {
  constructor(private readonly remote: Remote) {}

  publish(tag: string): void {
    this.remote.push(tag);
  }
}
