import jwt from "jsonwebtoken";

export function issueAccessToken(userId: string, key: string): string {
  return jwt.sign({ sub: userId }, key, { algorithm: "HS256", expiresIn: "7d" });
}
