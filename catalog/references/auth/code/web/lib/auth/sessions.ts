import { createHash, randomBytes } from "node:crypto";

export interface StoredRefreshToken {
  userId: string;
  family: string;
  expiresAt: number;
  used: boolean;
}

/** The port of the store of refresh tokens, keyed by the hash of each token. */
export interface RefreshTokenStore {
  save(tokenHash: string, stored: StoredRefreshToken): Promise<void>;
  get(tokenHash: string): Promise<StoredRefreshToken | undefined>;
  deleteFamily(family: string): Promise<void>;
}

/** The store in the memory of one process; several processes store the tokens in a database. */
export class InMemoryRefreshTokenStore implements RefreshTokenStore {
  private readonly byHash = new Map<string, StoredRefreshToken>();

  async save(tokenHash: string, stored: StoredRefreshToken): Promise<void> {
    this.byHash.set(tokenHash, stored);
  }

  async get(tokenHash: string): Promise<StoredRefreshToken | undefined> {
    return this.byHash.get(tokenHash);
  }

  async deleteFamily(family: string): Promise<void> {
    for (const [tokenHash, stored] of this.byHash) {
      if (stored.family === family) {
        this.byHash.delete(tokenHash);
      }
    }
  }
}

function hash(token: string): string {
  return createHash("sha256").update(token).digest("hex");
}

/** Opens sessions, rotates their refresh tokens and ends them. */
export class Sessions {
  private readonly store: RefreshTokenStore;
  private readonly refreshTokenTtlSeconds: number;

  constructor(store: RefreshTokenStore, refreshTokenTtlSeconds: number) {
    this.store = store;
    this.refreshTokenTtlSeconds = refreshTokenTtlSeconds;
  }

  /** Start a session for the user and return its first refresh token. */
  open(userId: string, now: number): Promise<string> {
    return this.issue(userId, randomBytes(16).toString("base64url"), now);
  }

  /** The user of the token and the token that replaces it, or null; a reused token ends the session. */
  async rotate(token: string, now: number): Promise<{ userId: string; token: string } | null> {
    const tokenHash = hash(token);
    const stored = await this.store.get(tokenHash);
    if (stored === undefined || stored.expiresAt <= now) {
      return null;
    }
    if (stored.used) {
      await this.store.deleteFamily(stored.family);
      return null;
    }
    await this.store.save(tokenHash, { ...stored, used: true });
    return { userId: stored.userId, token: await this.issue(stored.userId, stored.family, now) };
  }

  /** End the session of the refresh token, if it has one. */
  async close(token: string): Promise<void> {
    const stored = await this.store.get(hash(token));
    if (stored !== undefined) {
      await this.store.deleteFamily(stored.family);
    }
  }

  private async issue(userId: string, family: string, now: number): Promise<string> {
    const token = randomBytes(32).toString("base64url");
    const expiresAt = now + this.refreshTokenTtlSeconds;
    await this.store.save(hash(token), { userId, family, expiresAt, used: false });
    return token;
  }
}
