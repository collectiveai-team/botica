// The settings of authentication: the one module that reads the environment for it.

export interface AuthSettings {
  signingKey: Uint8Array;
  issuer: string;
  audience: string;
  accessTokenTtlSeconds: number;
  refreshTokenTtlSeconds: number;
}

function required(env: NodeJS.ProcessEnv, name: string): string {
  const value = env[name];
  if (!value) {
    throw new Error(`${name} is not set`);
  }
  return value;
}

function seconds(env: NodeJS.ProcessEnv, name: string, fallback: number): number {
  const value = Number(env[name] ?? fallback);
  if (!Number.isInteger(value) || value <= 0) {
    throw new Error(`${name} is not a positive number of seconds`);
  }
  return value;
}

/** Read and check the settings; an access token lives 900 seconds unless the environment says otherwise. */
export function loadSettings(env: NodeJS.ProcessEnv = process.env): AuthSettings {
  const signingKey = required(env, "APP_AUTH_SIGNING_KEY");
  if (signingKey.length < 32) {
    throw new Error("APP_AUTH_SIGNING_KEY is shorter than 32 characters");
  }
  return {
    signingKey: new TextEncoder().encode(signingKey),
    issuer: required(env, "APP_AUTH_ISSUER"),
    audience: required(env, "APP_AUTH_AUDIENCE"),
    accessTokenTtlSeconds: seconds(env, "APP_AUTH_ACCESS_TOKEN_TTL_SECONDS", 900),
    refreshTokenTtlSeconds: seconds(env, "APP_AUTH_REFRESH_TOKEN_TTL_SECONDS", 14 * 24 * 3600),
  };
}

let settings: AuthSettings | undefined;

/** The settings of the process, read once. */
export function getSettings(): AuthSettings {
  settings ??= loadSettings();
  return settings;
}
