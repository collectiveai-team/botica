import type { User } from "./users";

export function load(payload: Record<string, unknown>): User {
  return payload.user as unknown as User;
}
