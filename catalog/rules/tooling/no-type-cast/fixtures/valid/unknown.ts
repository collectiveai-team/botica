import { UserSchema, type User } from "./users";

export async function fetchUser(id: string): Promise<User> {
  const response = await fetch(`/users/${id}`);
  const body: unknown = await response.json();
  return UserSchema.parse(body);
}

export function message(error: unknown): string {
  const any = error instanceof Error ? error.message : "any error";
  return any;
}
