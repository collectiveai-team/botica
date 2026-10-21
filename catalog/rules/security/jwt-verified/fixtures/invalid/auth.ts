import jwt from "jsonwebtoken";
import { decodeJwt } from "jose";

export function claimsOf(token: string) {
  return jwt.decode(token);
}

export function lateClaimsOf(token: string, key: string) {
  return jwt.verify(token, key, { ignoreExpiration: true });
}

export function unsignedClaimsOf(token: string) {
  return jwt.verify(token, "", { algorithms: ["none"] });
}

export function joseClaimsOf(token: string) {
  return decodeJwt(token);
}
