import type { User } from "./users";

export function Name({ payload }: { payload: object }) {
  const user = (payload as unknown) as User;
  return <span>{user.name}</span>;
}
