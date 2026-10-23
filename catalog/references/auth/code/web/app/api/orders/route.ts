import { ACCESS_COOKIE, readCookie } from "../../../lib/auth/http.ts";
import { getSettings } from "../../../lib/auth/settings.ts";
import { verifyAccessToken } from "../../../lib/auth/tokens.ts";

/** The orders of the caller of the access cookie, or a 401. */
export async function GET(request: Request): Promise<Response> {
  const userId = await verifyAccessToken(readCookie(request, ACCESS_COOKIE) ?? "", getSettings());
  if (userId === null) {
    return Response.json({ detail: "invalid or expired token" }, { status: 401 });
  }
  return Response.json({ userId, orders: [] });
}
