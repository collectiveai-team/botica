import { getSettings } from "./settings.ts";
import { InMemoryRefreshTokenStore, Sessions } from "./sessions.ts";
import { issueAccessToken } from "./tokens.ts";

// `__Host-` binds the cookie to the host and to the path `/`; `__Secure-` lets the refresh cookie
// keep a narrower path. https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Set-Cookie#cookie_prefixes
export const ACCESS_COOKIE = "__Host-access";
export const REFRESH_COOKIE = "__Secure-refresh";
export const REFRESH_PATH = "/api/auth/refresh";

let sessions: Sessions | undefined;

/** The sessions of the process, over the store of refresh tokens. */
export function getSessions(): Sessions {
  sessions ??= new Sessions(new InMemoryRefreshTokenStore(), getSettings().refreshTokenTtlSeconds);
  return sessions;
}

export function nowSeconds(): number {
  return Math.floor(Date.now() / 1000);
}

/** The value of a cookie of the request, or undefined. */
export function readCookie(request: Request, name: string): string | undefined {
  for (const pair of (request.headers.get("cookie") ?? "").split(";")) {
    const [key, ...value] = pair.trim().split("=");
    if (key === name) {
      return value.join("=");
    }
  }
  return undefined;
}

function cookie(name: string, value: string, path: string, maxAge: number): string {
  return `${name}=${value}; Path=${path}; Max-Age=${maxAge}; HttpOnly; Secure; SameSite=Strict`;
}

/** The two cookies of a session: the access token, and the refresh token for its route only. */
export async function sessionCookies(
  userId: string,
  refreshToken: string,
  now: number,
): Promise<string[]> {
  const settings = getSettings();
  const accessToken = await issueAccessToken(userId, settings, now);
  return [
    cookie(ACCESS_COOKIE, accessToken, "/", settings.accessTokenTtlSeconds),
    cookie(REFRESH_COOKIE, refreshToken, REFRESH_PATH, settings.refreshTokenTtlSeconds),
  ];
}

/** The cookies that end a session in the browser. */
export function clearedCookies(): string[] {
  return [cookie(ACCESS_COOKIE, "", "/", 0), cookie(REFRESH_COOKIE, "", REFRESH_PATH, 0)];
}

/** Open a session for a user that the identity provider signed in, and return its cookies. */
export async function openSession(userId: string): Promise<string[]> {
  const now = nowSeconds();
  return sessionCookies(userId, await getSessions().open(userId, now), now);
}
