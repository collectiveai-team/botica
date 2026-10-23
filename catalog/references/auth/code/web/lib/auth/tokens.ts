import { jwtVerify, SignJWT } from "jose";

import type { AuthSettings } from "./settings.ts";

/** Sign an access token for the user that expires after the lifetime of the settings. */
export async function issueAccessToken(
  userId: string,
  settings: AuthSettings,
  now: number,
): Promise<string> {
  return new SignJWT({})
    .setProtectedHeader({ alg: "HS256" })
    .setSubject(userId)
    .setIssuer(settings.issuer)
    .setAudience(settings.audience)
    .setIssuedAt(now)
    .setExpirationTime(now + settings.accessTokenTtlSeconds)
    .sign(settings.signingKey);
}

/** The user of an access token of the service, or null when any check of the token fails. */
export async function verifyAccessToken(
  token: string,
  settings: AuthSettings,
): Promise<string | null> {
  try {
    const { payload } = await jwtVerify(token, settings.signingKey, {
      algorithms: ["HS256"],
      issuer: settings.issuer,
      audience: settings.audience,
      requiredClaims: ["exp", "iat", "sub"],
    });
    return payload.sub ?? null;
  } catch {
    return null;
  }
}
