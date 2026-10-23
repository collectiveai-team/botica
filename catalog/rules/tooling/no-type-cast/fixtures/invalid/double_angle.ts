import type { User } from "./users";

export function load(payload: Record<string, unknown>): User {
  return <User>(<unknown>payload.user);
}

export function plain(payload: Record<string, unknown>): User {
  return <User><never>payload.user;
}
