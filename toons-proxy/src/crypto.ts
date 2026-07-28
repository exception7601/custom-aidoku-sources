import CryptoJS from "crypto-js";

/**
 * Simple cache with TTL
 */
class SimpleCache {
  private store: Map<string, { value: unknown; expires: number }> = new Map();

  set(key: string, value: unknown, ttlSeconds: number): void {
    const expires = Date.now() + ttlSeconds * 1000;
    this.store.set(key, { value, expires });
  }

  get(key: string): unknown {
    const item = this.store.get(key);
    if (!item) return null;

    if (Date.now() > item.expires) {
      this.store.delete(key);
      return null;
    }

    return item.value;
  }

  has(key: string): boolean {
    return this.get(key) !== null;
  }

  clear(): void {
    this.store.clear();
  }
}

const tokenCache = new SimpleCache();

/**
 * Generate session ID
 */
function generateSession(): string {
  return (
    Math.random().toString(36).substring(2, 15) +
    Math.random().toString(36).substring(2, 15)
  );
}

/**
 * Generate passphrase based on current UTC time
 * Replicates ToonLivre's passphrase generation logic
 *
 * From bundle analysis:
 * - Uses Date.UTC(year, month, day, hour) as seed
 * - Applies MD5 hash
 * - Takes substring(0, 8) for the key
 */
function generatePassphrase(): string {
  const now = new Date();
  const utc = Date.UTC(
    now.getUTCFullYear(),
    now.getUTCMonth(),
    now.getUTCDate(),
    now.getUTCHours(),
  );
  const hash = CryptoJS.MD5(utc.toString()).toString();
  return hash.substring(0, 8);
}

/**
 * Fetch seed JWT from ToonLivre API
 */
async function fetchSeedJWT(): Promise<string> {
  const response = await fetch("https://toonlivre.net/api/seed", {
    headers: {
      "User-Agent":
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
      Accept: "application/json, text/plain, */*",
      "Accept-Language": "pt-BR,pt;q=0.9",
      Referer: "https://toonlivre.net/",
      Origin: "https://toonlivre.net",
    },
  });

  if (!response.ok) {
    throw new Error(`Seed endpoint returned ${response.status}`);
  }

  const data = (await response.json()) as { token?: string };
  if (!data?.token) {
    throw new Error("Token not found in seed response");
  }

  return data.token;
}

/**
 * Get authentication tokens and passphrase
 * Uses seed-jwt strategy (current ToonLivre implementation)
 */
export async function getAuthTokens(): Promise<{
  signature: string;
  verify: string;
  passphrase: string;
  session: string;
}> {
  const cacheKey = "current-tokens";

  // Check cache (25 second TTL)
  const cached = tokenCache.get(cacheKey) as {
    signature: string;
    verify: string;
    passphrase: string;
    session: string;
  } | null;
  if (cached) {
    console.log("[crypto] Using cached tokens");
    return cached;
  }

  console.log("[crypto] Generating new tokens");

  // Generate session and fetch JWT
  const session = generateSession();
  const passphrase = generatePassphrase();
  const signature = await fetchSeedJWT();

  const result = {
    signature, // JWT from /api/seed
    verify: session, // Random session ID
    passphrase, // UTC hour-based MD5 hash (first 8 chars)
    session,
  };

  // Cache for 25 seconds
  tokenCache.set(cacheKey, result, 25);
  console.log("[crypto] Tokens generated and cached (TTL: 25s)");

  return result;
}

/**
 * Decrypt data using Rabbit cipher
 */
export async function decryptData(encrypted: string): Promise<string> {
  const passphrase = generatePassphrase();
  const bytes = CryptoJS.Rabbit.decrypt(encrypted, passphrase);
  return bytes.toString(CryptoJS.enc.Utf8);
}

/**
 * Clear token cache
 */
export function clearTokenCache(): void {
  tokenCache.clear();
  console.log("[crypto] Token cache cleared");
}
