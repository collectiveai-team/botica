import type { User } from "./users";

export function load(payload: Record<string, unknown>): User {
  return <any>payload.user;
}

export function through(payload: Record<string, unknown>): User {
  return <User><any>payload.user;
}
