import {
  clearedCookies,
  getSessions,
  nowSeconds,
  readCookie,
  REFRESH_COOKIE,
  sessionCookies,
} from "../../../../lib/auth/http.ts";

function withCookies(status: number, cookies: string[]): Response {
  const headers = new Headers();
  for (const value of cookies) {
    headers.append("Set-Cookie", value);
  }
  return new Response(null, { status, headers });
}

/** Replace the refresh token of the cookie and set a new access token. */
export async function POST(request: Request): Promise<Response> {
  const now = nowSeconds();
  const rotated = await getSessions().rotate(readCookie(request, REFRESH_COOKIE) ?? "", now);
  if (rotated === null) {
    return withCookies(401, clearedCookies());
  }
  return withCookies(204, await sessionCookies(rotated.userId, rotated.token, now));
}

/** End the session of the refresh token of the cookie. */
export async function DELETE(request: Request): Promise<Response> {
  const token = readCookie(request, REFRESH_COOKIE);
  if (token !== undefined) {
    await getSessions().close(token);
  }
  return withCookies(204, clearedCookies());
}
