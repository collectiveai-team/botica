import jwt from "jsonwebtoken";
import { jwtVerify } from "jose";

export function claimsOf(token: string, key: string) {
  return jwt.verify(token, key, { algorithms: ["RS256"], ignoreExpiration: false });
}

export async function joseClaimsOf(token: string, key: CryptoKey) {
  return jwtVerify(token, key, { algorithms: ["ES256"] });
}

export const parsed = JSON.parse("{}");
export const text = Buffer.from("x").toString("base64");
