import jwt from "jsonwebtoken";

export function User({ token }: { token: string }) {
  const claims = jwt.decode(token) as { name: string };
  return <span>{claims.name}</span>;
}
