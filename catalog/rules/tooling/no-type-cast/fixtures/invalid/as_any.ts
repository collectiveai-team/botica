import type { User } from "./users";

export function load(payload: Record<string, unknown>): User {
  return payload.user as any;
}

export function through(payload: Record<string, unknown>): User {
  return payload.user as any as User;
}
