import { type User, isUser } from "./users";

export function load(payload: Record<string, unknown>): User {
  const user = payload.user;
  if (!isUser(user)) {
    throw new TypeError("payload.user is not a User");
  }
  return user;
}

export const ROLES = ["admin", "member"] as const;

export function input(event: Event): HTMLInputElement {
  return event.target as HTMLInputElement;
}
