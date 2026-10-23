import assert from "node:assert/strict";
import { beforeEach, describe, test } from "node:test";

process.env.APP_AUTH_SIGNING_KEY = "a-test-key-of-thirty-two-bytes!!";
process.env.APP_AUTH_ISSUER = "https://web.example.com/";
process.env.APP_AUTH_AUDIENCE = "orders";

const { DELETE, POST } = await import("../app/api/auth/refresh/route.ts");
const { GET } = await import("../app/api/orders/route.ts");
const { openSession } = await import("../lib/auth/http.ts");
const { InMemoryRefreshTokenStore } = await import("../lib/auth/sessions.ts");
const { loadSettings } = await import("../lib/auth/settings.ts");
const { issueAccessToken } = await import("../lib/auth/tokens.ts");

function value(cookies: string[], name: string): string {
  const found = cookies.find((cookie) => cookie.startsWith(`${name}=`));
  assert.ok(found, `no cookie ${name}`);
  return found.slice(name.length + 1).split(";")[0];
}

function request(path: string, cookies: Record<string, string>, method = "GET"): Request {
  const header = Object.entries(cookies)
    .map(([name, cookieValue]) => `${name}=${cookieValue}`)
    .join("; ");
  return new Request(`https://web.example.com${path}`, { method, headers: { cookie: header } });
}

describe("the settings", () => {
  test("give an access token 900 seconds when the environment names no lifetime", () => {
    const env = { ...process.env, APP_AUTH_ACCESS_TOKEN_TTL_SECONDS: undefined };
    assert.equal(loadSettings(env).accessTokenTtlSeconds, 900);
  });

  test("take the lifetime of the environment, and reject one that is not above 0", () => {
    const env = { ...process.env, APP_AUTH_ACCESS_TOKEN_TTL_SECONDS: "3600" };
    assert.equal(loadSettings(env).accessTokenTtlSeconds, 3600);
    assert.throws(
      () => loadSettings({ ...process.env, APP_AUTH_ACCESS_TOKEN_TTL_SECONDS: "0" }),
      /not a positive number of seconds/,
    );
  });
});

describe("a session", () => {
  let cookies: string[];

  beforeEach(async () => {
    cookies = await openSession("ana");
  });

  test("sets both cookies HttpOnly, Secure and SameSite, the refresh one for its route only", () => {
    for (const cookie of cookies) {
      assert.match(cookie, /; HttpOnly; Secure; SameSite=Strict$/);
    }
    assert.match(cookies[0], /^__Host-access=[^;]+; Path=\/; Max-Age=900;/);
    assert.match(cookies[1], /^__Secure-refresh=[^;]+; Path=\/api\/auth\/refresh;/);
  });

  test("lets the access cookie reach the route", async () => {
    const response = await GET(request("/api/orders", { "__Host-access": value(cookies, "__Host-access") }));

    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { userId: "ana", orders: [] });
  });

  test("replaces the refresh cookie, and a replayed one ends the session", async () => {
    const first = value(cookies, "__Secure-refresh");

    const refreshed = await POST(request("/api/auth/refresh", { "__Secure-refresh": first }, "POST"));
    const second = value(refreshed.headers.getSetCookie(), "__Secure-refresh");

    assert.equal(refreshed.status, 204);
    assert.notEqual(second, first);
    const replayed = await POST(request("/api/auth/refresh", { "__Secure-refresh": first }, "POST"));
    assert.equal(replayed.status, 401);
    const after = await POST(request("/api/auth/refresh", { "__Secure-refresh": second }, "POST"));
    assert.equal(after.status, 401);
  });

  test("ends when the browser signs out", async () => {
    const token = value(cookies, "__Secure-refresh");

    await DELETE(request("/api/auth/refresh", { "__Secure-refresh": token }, "DELETE"));

    const refreshed = await POST(request("/api/auth/refresh", { "__Secure-refresh": token }, "POST"));
    assert.equal(refreshed.status, 401);
  });
});

describe("the store of refresh tokens", () => {
  test("marks a token used once", async () => {
    const store = new InMemoryRefreshTokenStore();
    await store.save("hash", { userId: "ana", family: "family", expiresAt: 0, used: false });

    assert.deepEqual([await store.markUsed("hash"), await store.markUsed("hash")], [true, false]);
  });
});

describe("the route of orders", () => {
  test("answers 401 without an access cookie", async () => {
    assert.equal((await GET(request("/api/orders", {}))).status, 401);
  });

  test("answers 401 to an expired access token", async () => {
    const settings = loadSettings();
    const expired = await issueAccessToken("ana", settings, Math.floor(Date.now() / 1000) - 3600);

    assert.equal((await GET(request("/api/orders", { "__Host-access": expired }))).status, 401);
  });
});
