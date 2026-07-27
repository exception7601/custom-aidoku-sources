import axios from "axios";

interface CacheEntry {
  data: unknown;
  expiresAt: number;
  cachedAt: number;
  requestTime: number;
}

interface TokenResponse {
  token: string;
  expiresIn?: number;
}

export class TokenManager {
  private token: string | null = null;
  private tokenExpiresAt = 0;
  private tokenFetchedAt = 0;
  private cache: Map<string, CacheEntry> = new Map();
  private readonly TOKEN_ENDPOINT = "https://toonlivre.net/api/tokens";
  private readonly TOKEN_CACHE_TTL = 3600; // 1 hour
  readonly REQUEST_CACHE_TTL = 20; // 20 seconds

  async getToken(): Promise<{ token: string; expiresIn: number }> {
    const now = Date.now();

    // Return cached token if still valid
    if (
      this.token &&
      this.tokenExpiresAt > now &&
      this.tokenFetchedAt + 5000 > now
    ) {
      const expiresIn = Math.floor((this.tokenExpiresAt - now) / 1000);
      console.log(
        "[token] using cached token, expires in",
        expiresIn,
        "seconds",
      );
      return { token: this.token, expiresIn };
    }

    try {
      console.log("[token] fetching new token from API");
      const startTime = Date.now();

      const response = await axios.post<TokenResponse>(this.TOKEN_ENDPOINT, {});

      const requestTime = Date.now() - startTime;
      this.token = response.data.token;
      this.tokenFetchedAt = Date.now();
      this.tokenExpiresAt =
        now + (response.data.expiresIn || this.TOKEN_CACHE_TTL) * 1000;

      const expiresIn = response.data.expiresIn || this.TOKEN_CACHE_TTL;
      console.log(
        "[token] new token acquired in",
        requestTime,
        "ms, expires in",
        expiresIn,
        "seconds",
      );

      return { token: this.token, expiresIn };
    } catch (error) {
      console.error("[token] failed to fetch token:", error);
      throw new Error("Failed to obtain API token");
    }
  }

  getFromCache(key: string): unknown | null {
    const entry = this.cache.get(key);

    if (!entry) {
      return null;
    }

    const now = Date.now();
    if (entry.expiresAt < now) {
      console.log("[cache] entry expired for key:", key);
      this.cache.delete(key);
      return null;
    }

    const cacheAge = now - entry.cachedAt;
    console.log(
      "[cache] hit for key",
      key,
      "age",
      cacheAge,
      "ms, original request",
      entry.requestTime,
      "ms",
    );
    return entry.data;
  }

  setCache(
    key: string,
    data: unknown,
    requestTime: number,
    ttlSeconds = 20,
  ): void {
    const now = Date.now();
    this.cache.set(key, {
      data,
      expiresAt: now + ttlSeconds * 1000,
      cachedAt: now,
      requestTime,
    });

    console.log(
      "[cache] stored key",
      key,
      "ttl",
      ttlSeconds,
      "s, original request",
      requestTime,
      "ms",
    );
  }

  getCacheStats(): {
    size: number;
    keys: string[];
    tokenExpiresIn: number;
  } {
    this.clearExpiredEntries();
    const now = Date.now();
    const tokenExpiresIn = Math.max(
      0,
      Math.floor((this.tokenExpiresAt - now) / 1000),
    );

    return {
      size: this.cache.size,
      keys: Array.from(this.cache.keys()),
      tokenExpiresIn,
    };
  }

  clearCache(): void {
    console.log("[cache] clearing all cached entries");
    this.cache.clear();
  }

  private clearExpiredEntries(): void {
    const now = Date.now();
    let cleared = 0;

    for (const [key, entry] of this.cache.entries()) {
      if (entry.expiresAt < now) {
        this.cache.delete(key);
        cleared++;
      }
    }

    if (cleared > 0) {
      console.log("[cache] cleared", cleared, "expired entries");
    }
  }
}

export const tokenManager = new TokenManager();
